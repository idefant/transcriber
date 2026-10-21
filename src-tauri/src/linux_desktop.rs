//! Интеграция с Hyprland Lua: приватный транспорт действий и принадлежащие
//! приложению бинды. Чужие сочетания никогда не удаляются и не переопределяются.

use crate::{
    dictation,
    error::{AppError, AppResult},
    settings::{self, AppSettings, TriggerMode},
    shortcut_hook::{self, ShortcutState},
};
use std::{
    collections::{HashMap, HashSet},
    fs,
    os::unix::{fs::PermissionsExt, net::UnixDatagram},
    path::PathBuf,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex, OnceLock,
    },
    time::Duration,
};

static INSTANCE: OnceLock<String> = OnceLock::new();
static APP: OnceLock<tauri::AppHandle> = OnceLock::new();
static REGISTRATION: Mutex<()> = Mutex::new(());
static CANCEL_ARMED: AtomicBool = AtomicBool::new(false);
static PAUSE_ARMED: AtomicBool = AtomicBool::new(false);
static ENABLED: AtomicBool = AtomicBool::new(false);
static STOPPING: AtomicBool = AtomicBool::new(false);
static LAST_REGISTRATION: OnceLock<Mutex<String>> = OnceLock::new();

/** Пространство имён независимых профилей приложения. */
pub(crate) fn namespace() -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    INSTANCE
        .get()
        .map(String::as_str)
        .unwrap_or("com.transcriber.desktop")
        .hash(&mut hasher);
    format!("transcriber_{:016x}", hasher.finish())
}
fn lua_string(value: &str) -> String {
    serde_json::to_string(value).expect("string is serializable")
}
fn socket_path() -> AppResult<PathBuf> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .ok_or_else(|| AppError::from("XDG_RUNTIME_DIR is required for Hyprland controls"))?;
    Ok(PathBuf::from(runtime).join(format!("{}.sock", namespace())))
}

/** Команды не запускают второй GUI-процесс и не показывают главное окно. */
pub fn run_control_client(identifier: &str) -> bool {
    let _ = INSTANCE.set(identifier.to_owned());
    let args: Vec<String> = std::env::args().collect();
    let Some(position) = args.iter().position(|arg| arg == "--action") else {
        return false;
    };
    let result = (|| -> AppResult<()> {
        let action = args
            .get(position + 1)
            .ok_or_else(|| AppError::from("--action requires a value"))?;
        if ![
            "pressed",
            "released",
            "cancel",
            "pause",
            "copy",
            "paste",
            "repeat",
            "paste-pressed",
            "paste-released",
        ]
        .contains(&action.as_str())
        {
            return Err(AppError::from("Unknown Transcriber action"));
        }
        let socket = UnixDatagram::unbound()?;
        socket.connect(socket_path()?)?;
        let identity = args
            .iter()
            .position(|arg| arg == "--activation")
            .and_then(|position| args.get(position + 1))
            .map(String::as_str)
            .unwrap_or("manual");
        if identity.len() > 100
            || !identity
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return Err(AppError::from("Invalid activation identity"));
        }
        socket.send(format!("{action}:{identity}").as_bytes())?;
        Ok(())
    })();
    if let Err(error) = result {
        eprintln!("{}", error.into_message());
        std::process::exit(1);
    }
    true
}

/** Слушатель сериализует нажатие, отпускание и отмену вне потока GUI. */
pub fn initialize(app: tauri::AppHandle) -> AppResult<()> {
    if APP.set(app.clone()).is_err() {
        return Ok(());
    }
    let path = socket_path()?;
    // Single-instance уже исключил живой второй экземпляр; оставшийся socket — от аварийного выхода.
    if path.exists() {
        fs::remove_file(&path)?;
    }
    let socket = UnixDatagram::bind(&path)?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    let handle = app.clone();
    std::thread::spawn(move || {
        let mut bytes = [0u8; 128];
        let mut pressed: Option<String> = None;
        let mut released = std::collections::VecDeque::new();
        let mut paste_pressed = false;
        while let Ok(count) = socket.recv(&mut bytes) {
            let message = String::from_utf8_lossy(&bytes[..count]);
            let (action, identity) = message.split_once(':').unwrap_or((&message, "manual"));
            if shortcut_hook::is_capture_active() && action != "released" {
                continue;
            }
            match action {
                "pressed" if pressed.as_deref() != Some(identity) => {
                    let hold = settings::load_app_settings(&handle)
                        .is_ok_and(|s| matches!(s.trigger_mode(), TriggerMode::Hold));
                    // Короткое нажатие может породить release-процесс раньше press-процесса.
                    if hold && released.iter().any(|id| id == identity) {
                        continue;
                    }
                    pressed = Some(identity.to_owned());
                    dictation::handle_shortcut_event(&handle, ShortcutState::Pressed);
                }
                "released" => {
                    if pressed.as_deref() == Some(identity) {
                        pressed = None;
                        dictation::handle_shortcut_event(&handle, ShortcutState::Released);
                    }
                    released.push_back(identity.to_owned());
                    if released.len() > 1024 {
                        released.pop_front();
                    }
                }
                "cancel" if CANCEL_ARMED.load(Ordering::Relaxed) => {
                    dictation::handle_cancel_shortcut(&handle)
                }
                "pause" if PAUSE_ARMED.load(Ordering::Relaxed) => {
                    dictation::handle_pause_shortcut(&handle)
                }
                "copy" => dictation::handle_copy_latest_shortcut(&handle),
                "paste-pressed" => paste_pressed = true,
                "paste-released" if paste_pressed => {
                    paste_pressed = false;
                    dictation::handle_paste_latest_shortcut(&handle);
                }
                "paste" => dictation::handle_paste_latest_shortcut(&handle),
                "repeat" => dictation::handle_repeat_latest_shortcut(&handle),
                _ => {}
            }
        }
    });
    // После перезагрузки Lua-конфига исчезает registry; восстанавливаем свои бинды.
    std::thread::spawn(move || {
        while !STOPPING.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_secs(2));
            if let Ok(config) = settings::load_app_settings(&app) {
                if let Err(error) = refresh(&config) {
                    eprintln!("Hyprland shortcuts: {}", error.into_message());
                }
            }
        }
    });
    Ok(())
}

/** Передаёт один аргумент IPC без shell-интерполяции и проверяет ошибки Lua. */
pub(crate) fn hyprctl(args: &[&str]) -> AppResult<String> {
    let output = Command::new("hyprctl").args(args).output()?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if !output.status.success() || text.starts_with("error") || text.contains("Lua error") {
        return Err(AppError::from(format!(
            "Hyprland: {} {}",
            text,
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(text)
}

struct Binding {
    id: String,
    combo: String,
    key: String,
    keycode: Option<u64>,
    mask: u64,
    action: &'static str,
    release: bool,
}

fn binding(id: &str, value: &str, action: &'static str, release: bool) -> AppResult<Binding> {
    let mut parts = Vec::new();
    let mut mask = 0;
    let mut key = String::new();
    for token in value.split('+') {
        match token {
            "LCtrl" | "RCtrl" | "LAlt" | "RAlt" | "LShift" | "RShift" | "LWin" | "RWin" => return Err(AppError::from("Hyprland shortcuts do not distinguish modifier sides; assign the combination again on Linux")),
            "Ctrl" => {
                parts.push("CTRL".to_string());
                mask |= 4;
            }
            "Alt" => {
                parts.push("ALT".to_string());
                mask |= 8;
            }
            "Shift" => {
                parts.push("SHIFT".to_string());
                mask |= 1;
            }
            "Win" => {
                parts.push("SUPER".to_string());
                mask |= 64;
            }
            other => {
                key = match other {
                    "Enter" => "Return",
                    "ArrowUp" => "Up",
                    "ArrowDown" => "Down",
                    "ArrowLeft" => "Left",
                    "ArrowRight" => "Right",
                    "PageUp" => "Prior",
                    "PageDown" => "Next",
                    "Backspace" => "BackSpace",
                    other => other,
                }
                .to_owned()
            }
        }
    }
    if key.is_empty() {
        return Err(AppError::from("Hyprland shortcut requires a main key"));
    }
    parts.push(key.clone());
    let keycode = key
        .strip_prefix('F')
        .and_then(|number| number.parse::<u64>().ok())
        .filter(|number| (13..=24).contains(number))
        .map(|number| 178 + number);
    Ok(Binding {
        id: id.to_owned(),
        combo: parts.join(" + "),
        key,
        keycode,
        mask,
        action,
        release,
    })
}

fn bindings(config: &AppSettings, include_inactive: bool) -> AppResult<Vec<Binding>> {
    let mut result = vec![binding("dictation", config.hotkey(), "pressed", false)?];
    for (id, value, action, active) in [
        ("copy", config.copy_latest_hotkey(), "copy", true),
        ("paste", config.paste_latest_hotkey(), "paste", true),
        ("repeat", config.repeat_latest_hotkey(), "repeat", true),
        (
            "cancel",
            config.cancel_hotkey(),
            "cancel",
            CANCEL_ARMED.load(Ordering::Relaxed),
        ),
        (
            "pause",
            config.pause_hotkey(),
            "pause",
            PAUSE_ARMED.load(Ordering::Relaxed)
                && matches!(config.trigger_mode(), TriggerMode::Press),
        ),
    ] {
        if !value.is_empty() && (active || include_inactive) {
            result.push(binding(id, value, action, action == "paste")?);
        }
    }
    Ok(result)
}

// Стандартная evdev-карта назначает дополнительным F-клавишам мультимедийные
// символы. Учитываем их консервативно, даже если раскладка вернула обычный F-символ.
fn physical_key_symbols(keycode: u64) -> &'static [&'static str] {
    match keycode {
        191 => &["XF86Tools"],
        192 => &["XF86Launch5"],
        193 => &["XF86Launch6"],
        194 => &["XF86Launch7"],
        195 => &["XF86Launch8"],
        196 => &["XF86Launch9"],
        198 => &["XF86AudioMicMute"],
        199 => &["XF86TouchpadToggle"],
        200 => &["XF86TouchpadOn"],
        201 => &["XF86TouchpadOff", "XF86Assistant"],
        202 => &["XF86TouchpadToggle"],
        _ => &[],
    }
}

fn physical_keycode(value: &str) -> Option<u64> {
    value
        .rsplit('+')
        .next()?
        .trim()
        .strip_prefix("code:")?
        .parse()
        .ok()
}

fn check_conflicts(wanted: &[Binding]) -> AppResult<()> {
    let current: serde_json::Value = serde_json::from_str(&hyprctl(&["binds", "-j"])?)?;
    // В Hyprland 0.56 только текстовый вывод сохраняет display_key Lua-бинда.
    let plain = if wanted.iter().any(|item| item.keycode.is_some())
        && current.as_array().is_some_and(|bindings| {
            bindings.iter().any(|binding| {
                binding["key"].as_str() == Some("") && binding["keycode"].as_u64() == Some(0)
            })
        }) {
        hyprctl(&["binds"])?
    } else {
        String::new()
    };
    let display_bindings: Vec<HashMap<&str, &str>> = plain
        .split("\n\n")
        .map(|block| {
            block
                .lines()
                .filter_map(|line| line.trim().split_once(':'))
                .map(|(name, value)| (name.trim(), value.trim()))
                .collect()
        })
        .collect();
    let prefix = format!("{}:", namespace());
    let mut unique = HashSet::new();
    for item in wanted {
        if !unique.insert((item.mask, item.key.to_ascii_lowercase())) {
            return Err(AppError::from(format!(
                "Shortcut {} is assigned to multiple Transcriber actions",
                item.combo
            )));
        }
        if let Some(existing) = current.as_array() {
            for existing in existing {
                let description = existing["description"].as_str().unwrap_or_default();
                let key = existing["key"].as_str().unwrap_or_default();
                let keycode = existing["keycode"].as_u64().unwrap_or_default();
                let mut display_codes = display_bindings
                    .iter()
                    .filter(|display| {
                        ["dispatcher", "arg", "description", "submap"]
                            .iter()
                            .all(|field| {
                                display.get(field).copied()
                                    == Some(existing[*field].as_str().unwrap_or_default())
                            })
                            && display
                                .get("modmask")
                                .and_then(|mask| mask.parse::<u64>().ok())
                                == existing["modmask"].as_u64()
                    })
                    .map(|display| display.get("key").and_then(|key| physical_keycode(key)));
                let display_code = display_codes.next().flatten().filter(|code| {
                    // При изменении биндов между снимками неоднозначность не
                    // должна становиться доказательством отсутствия конфликта.
                    display_codes.all(|other| other == Some(*code))
                });
                let matches = !description.starts_with(&prefix)
                    && existing["modmask"].as_u64() == Some(item.mask)
                    && (existing["key"].as_str().is_some_and(|key| {
                        key.eq_ignore_ascii_case(&item.key)
                            || item.keycode.is_some_and(|code| {
                                physical_key_symbols(code)
                                    .iter()
                                    .any(|symbol| key.eq_ignore_ascii_case(symbol))
                            })
                    }) || item.keycode.is_some_and(|code| {
                        keycode == code
                            || physical_keycode(key) == Some(code)
                            || (key.is_empty()
                                && keycode == 0
                                && display_code.is_none_or(|other| other == code))
                    }) || existing["catch_all"].as_bool() == Some(true));
                if matches && !available_managed_binding(existing, item)? {
                    return Err(AppError::from(format!(
                        "Shortcut {} is already used by Hyprland; choose another combination",
                        item.combo
                    )));
                }
            }
        }
    }
    Ok(())
}

// JSON binds не включает enabled и может скрывать физическую клавишу Lua-бинда.
// Проверяем точный handle другого экземпляра по description, mask, dispatcher и arg.
fn available_managed_binding(existing: &serde_json::Value, wanted: &Binding) -> AppResult<bool> {
    let description = existing["description"].as_str().unwrap_or_default();
    let Some((namespace, _)) = description.split_once(':') else {
        return Ok(false);
    };
    let Some(hash) = namespace.strip_prefix("transcriber_") else {
        return Ok(false);
    };
    if hash.len() != 16 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Ok(false);
    }
    let mask = existing["modmask"].as_u64().unwrap_or_default();
    let key = existing["key"].as_str().unwrap_or_default();
    let keycode = existing["keycode"].as_u64().unwrap_or_default();
    let dispatcher = existing["dispatcher"].as_str().unwrap_or_default();
    let arg = existing["arg"].as_str().unwrap_or_default();
    if dispatcher != "__lua" || arg.is_empty() {
        return Ok(false);
    }
    let different_key = wanted.keycode.map_or("false".to_owned(), |code| {
        format!("(function() local k=(b.display_key or ''):match('([^+]+)$') or ''; local c=tonumber(k:match('^%s*code:(%d+)%s*$')); return c~=nil and c~={code} end)()")
    });
    let code = format!("local found=false; local r=_G[{}]; if type(r)=='table' and type(r.binds)=='table' then for _,b in pairs(r.binds) do if b.description=={} and b.modmask=={mask} and b.handler=={} and b.arg=={} and b.key=={} and b.keycode=={keycode} then found=true; if b:is_enabled() and not {different_key} then return false end end end end; return found", lua_string(namespace), lua_string(description), lua_string(dispatcher), lua_string(arg), lua_string(key));
    Ok(hyprctl(&["repl", &code])?.trim() == "true")
}

/** Проверяет конфликты до сохранения настроек, включая временно неактивные действия. */
pub fn validate_settings(config: &AppSettings) -> AppResult<()> {
    check_conflicts(&bindings(config, true)?)
}

/** Применяет настройки к текущей сессии; системные конфиги не редактируются. */
pub fn refresh(config: &AppSettings) -> AppResult<()> {
    let _lock = REGISTRATION
        .lock()
        .map_err(|e| AppError::from(e.to_string()))?;
    let wanted =
        if shortcut_hook::is_capture_active() || crate::keyboard::is_synthetic_paste_active() {
            Vec::new()
        } else {
            bindings(config, false)?
        };
    check_conflicts(&wanted)?;
    let pid = std::process::id();
    let starttime = fs::read_to_string("/proc/self/stat")?
        .rsplit_once(") ")
        .and_then(|(_, fields)| fields.split_whitespace().nth(19))
        .unwrap_or("")
        .to_owned();
    let mut code = format!(
        "local r=_G.{0}; if r and r.pid~={1} then for _,b in pairs(r.binds) do b:set_enabled(false) end; r=nil end; if not r then r={{binds={{}},counter=0,epoch={2}..'-'..tostring(math.random(1000000000)),pid={1}}}; _G.{0}=r; r.watchdog=hl.timer(function() local f=io.open('/proc/{1}/stat','r'); local alive=false; if f then local stat=f:read('*a'); f:close(); local tail=stat:match('^%d+ %b() (.*)'); if tail then local n=0; for field in tail:gmatch('%S+') do n=n+1; if n==20 then alive=field=={3}; break end end end end; if not alive or _G.{0}~=r then for _,b in pairs(r.binds) do b:set_enabled(false) end; r.watchdog:set_enabled(false) end end,{{timeout=2000,type='repeat'}}) end; ",
        namespace(), pid, lua_string(&pid.to_string()), lua_string(&starttime)
    );
    code.push_str("for _,b in pairs(r.binds) do b:set_enabled(false) end; ");
    let executable = std::env::current_exe()?;
    let quoted_exe = format!("'{}'", executable.to_string_lossy().replace('\'', "'\\''"));
    for item in &wanted {
        for (suffix, action, release, ignore_mods) in if item.action == "pressed" {
            vec![
                ("press", "pressed", false, false),
                ("release", "released", true, true),
            ]
        } else if item.action == "paste" {
            vec![
                ("press", "paste-pressed", false, false),
                ("release", "paste-released", true, true),
            ]
        } else {
            vec![("action", item.action, item.release, false)]
        } {
            // Hyprland сопоставляет code: с XKB-кодом (evdev + 8), независимо
            // от XF86-символа дополнительной F-клавиши в текущей раскладке.
            let key = item.keycode.map(|code| format!("code:{code}"));
            let key = match (ignore_mods, key) {
                (true, key) => key.unwrap_or_else(|| item.key.clone()),
                (false, Some(key)) => {
                    item.combo.strip_suffix(&item.key).unwrap_or("").to_owned() + &key
                }
                (false, None) => item.combo.clone(),
            };
            let id = lua_string(&format!("{}:{}:{}", item.id, item.combo, suffix));
            let description = lua_string(&format!("{}:{}", namespace(), item.id));
            let command = lua_string(&format!("{quoted_exe} --action {action}"));
            let dispatcher = if action == "pressed" {
                format!("function() r.counter=r.counter+1; r.active=r.epoch..'-'..tostring(r.counter); hl.exec_cmd({command}..' --activation '..r.active) end")
            } else if action == "released" {
                format!("function() if r.active then hl.exec_cmd({command}..' --activation '..r.active); r.active=nil end end")
            } else {
                format!("hl.dsp.exec_cmd({command})")
            };
            code.push_str(&format!("if not r.binds[{id}] then r.binds[{id}]=hl.bind({},{dispatcher},{{release={release},ignore_mods={ignore_mods},non_consuming={ignore_mods},description={description}}}) end; r.binds[{id}]:set_enabled(true); ", lua_string(&key)));
        }
    }
    // Не пересоздаём бинды при каждом опросе, но обнаруживаем reload по отсутствию registry.
    let signature = code.clone();
    let cached = LAST_REGISTRATION.get_or_init(|| Mutex::new(String::new()));
    let mut previous = cached.lock().map_err(|e| AppError::from(e.to_string()))?;
    if *previous == signature {
        code = format!("if not _G.{} then {} end", namespace(), code);
    }
    hyprctl(&["eval", &code])?;
    *previous = signature;
    // Во время собственной вставки DOM fallback тоже должен оставаться отключённым.
    let enabled = !wanted.is_empty()
        || (crate::keyboard::is_synthetic_paste_active() && ENABLED.load(Ordering::Relaxed));
    if ENABLED.swap(enabled, Ordering::Relaxed) != enabled {
        if let Some(app) = APP.get() {
            use tauri::Emitter;
            let _ = app.emit("native-shortcuts-enabled", enabled);
        }
    }
    Ok(())
}

fn refresh_current() {
    if let Some(app) = APP.get() {
        if let Ok(settings) = settings::load_app_settings(app) {
            if let Err(error) = refresh(&settings) {
                eprintln!("Hyprland: {}", error.into_message());
            }
        }
    }
}

/** Временные сочетания захватываются только пока соответствующее действие доступно. */
pub fn set_cancel_armed(active: bool) {
    CANCEL_ARMED.store(active, Ordering::Relaxed);
    refresh_current();
}
/** В режиме удержания вызывающий код не вооружает паузу. */
pub fn set_pause_armed(active: bool) {
    PAUSE_ARMED.store(active, Ordering::Relaxed);
    refresh_current();
}
/** Даёт webview получить новую комбинацию, отключая собственные глобальные бинды. */
pub fn capture_changed() {
    refresh_current();
}
/** Отключает только свои handles перед штатным выходом. */
pub fn shutdown() {
    STOPPING.store(true, Ordering::Relaxed);
    let _ = hyprctl(&[
        "eval",
        &format!(
            "local r=_G.{}; if r then for _,b in pairs(r.binds) do b:set_enabled(false) end end",
            namespace()
        ),
    ]);
    if let Ok(path) = socket_path() {
        let _ = fs::remove_file(path);
    }
}

/** Позволяет DOM-пути работать, если глобальная интеграция недоступна. */
pub fn shortcuts_enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}
