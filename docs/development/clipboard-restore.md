# Clipboard Snapshot and Restore

Dictation paste stages the transcribed text on the clipboard, sends a synthetic `Ctrl+V`, then puts the previous clipboard contents back. `src-tauri/src/keyboard.rs` implements that snapshot/restore cycle on top of the [`clipboard-win`](https://docs.rs/clipboard-win) crate.

This note records why the code looks the way it does. The Windows clipboard is not a `format -> bytes` dictionary, and the two non-obvious rules below were both found by pasting an image with a known marker pixel and checking where that pixel landed afterwards.

## Why the whole clipboard is snapshotted

The original implementation read only `CF_UNICODETEXT`. For anything else — an image, a file list, HTML — the read returned nothing and the restore path called `EmptyClipboard`, silently destroying the user's clipboard. Copying an image and then dictating made the image unpastable.

The snapshot therefore enumerates every available format and copies each one's memory block verbatim.

## Which formats are skipped

`is_restorable_format` rejects formats whose clipboard handle is not an `HGLOBAL` memory block and therefore cannot be copied byte for byte:

- `CF_BITMAP`, `CF_PALETTE` — GDI handles. Images survive anyway, because Windows synthesizes them from the restored `CF_DIB`.
- `CF_METAFILEPICT`, `CF_ENHMETAFILE` — GDI handles for vector metafiles. These are genuinely lost unless the source also published a raster or memory-backed format.
- `CF_OWNERDISPLAY`, the `CF_DSP*` family — owner-drawn display formats.
- `CF_PRIVATEFIRST..CF_PRIVATELAST` and `CF_GDIOBJFIRST..CF_GDIOBJLAST` — application-private ranges whose memory the system does not manage.

Formats whose `GetClipboardData` returns a null handle are recorded as present with no payload and re-set the same way, which is how marker formats such as `ExcludeClipboardContentFromMonitorProcessing` carry meaning.

OLE-published content is snapshotted as raw Win32 formats only. The live `IDataObject` of the source application is not recreated, so an embedded object or a mail attachment loses its OLE identity even though its raw formats come back.

## Rule 1: keep `CF_DIB`, drop `CF_DIBV5`

Windows enumerates an image as both `CF_DIB` and `CF_DIBV5` regardless of which one the source placed, because each is synthesized from the other. Reading each costs a full-size conversion inside Windows — roughly 50 ms apiece for a 4K screenshot — so only one is worth capturing.

It must be `CF_DIB`. A `BITMAPINFOHEADER` is always followed by the three `BI_BITFIELDS` masks, so the pixel offset is unambiguous. A `BITMAPV5HEADER` already carries those masks inside the header, yet the buffer Windows synthesizes still appends 12 mask bytes after it. Writing those bytes back as a native `CF_DIBV5` makes readers treat the masks as pixel data, which shifts the entire image three pixels sideways.

Keeping `CF_DIB` also halves the snapshot cost. For a 3840x2160 screenshot the snapshot drops from about 87 ms to about 48 ms and from 66 MB to 33 MB.

## Rule 2: force `CF_BITMAP` synthesis after restoring an image

The original clipboard usually carried a real `HBITMAP`, which cannot be copied into a snapshot. After a restore, `CF_BITMAP` therefore has to be synthesized. Windows derives it from whichever DIB format is currently materialized, and its `CF_DIBV5` path has the same mask-offset bug described above.

So if a paste target reads `CF_DIBV5` before anything reads `CF_BITMAP`, the synthesized bitmap comes out shifted three pixels sideways. `force_bitmap_synthesis` reads `CF_BITMAP` immediately after the restore, which pins the correct `CF_DIB`-derived handle in the clipboard's cache before any consumer can poison it.

This runs after the synthetic `Ctrl+V`, so it does not delay the paste.

## Two things `clipboard-win` does not cover

`Clipboard::new_attempts` retries `OpenClipboard` but only yields the scheduler slice between tries. A paste target can hold the clipboard open for longer than that, so `open_clipboard` runs its own retry loop with real 30 ms sleeps.

`raw::set_without_clear` returns `Ok` without writing anything when the data slice is empty, and the crate exposes no way to place a null clipboard handle. Marker formats need exactly that, so `set_empty_format` calls `SetClipboardData` directly.

## Testing this area

Format-level assertions are not enough: a three-pixel shift keeps every format, size, and checksum intact. Verify image fidelity by copying a bitmap that has a single marker pixel at a known coordinate, running the paste cycle, and reading the pixel back through an external consumer such as `System.Windows.Forms.Clipboard::GetImage`.

Check both orders: a plain read after the restore, and a read that materializes `CF_DIBV5` first. Seed the clipboard from a separate process, and verify the seed actually succeeded — `SetImage` fails transiently under clipboard contention.
