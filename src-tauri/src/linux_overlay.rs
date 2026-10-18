//! Layer-shell сохраняет клавиатурный фокус приложения за интерактивной карточкой.

use gtk::prelude::*;
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use tauri::WebviewWindow;

use crate::{
    error::{AppError, AppResult},
    linux_desktop,
};

fn quote(value: &str) -> String {
    serde_json::to_string(value).expect("string is serializable")
}

/** Уникальный заголовок отличает собственные окна при диагностике. */
pub fn title(label: &str) -> String {
    format!("{}_{label}", linux_desktop::namespace())
}

// GTK-объекты создаются и используются только в главном потоке; через канал
// передаётся результат, но никогда сам ApplicationWindow или GdkMonitor.
fn with_native(
    window: &WebviewWindow,
    action: impl FnOnce(&gtk::ApplicationWindow) -> AppResult<()> + Send + 'static,
) -> AppResult<()> {
    if gtk::is_initialized_main_thread() {
        return action(&window.gtk_window()?);
    }
    let handle = window.clone();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    window.run_on_main_thread(move || {
        let result = handle
            .gtk_window()
            .map_err(AppError::from)
            .and_then(|native| action(&native));
        let _ = sender.send(result);
    })?;
    receiver
        .recv()
        .map_err(|error| AppError::from(error.to_string()))?
}

/** Назначает Wayland-роль скрытому окну до первого realize и показа. */
pub fn initialize(window: &WebviewWindow, shadow: bool) -> AppResult<()> {
    with_native(window, move |native| {
        if native.is_realized() {
            return Err(AppError::from(
                "Overlay GTK window was realized before layer-shell initialization",
            ));
        }
        if !gtk_layer_shell::is_supported() {
            return Err(AppError::from(
                "Wayland compositor does not support layer-shell",
            ));
        }
        native.init_layer_shell();
        native.set_namespace(&format!("{}_overlay", linux_desktop::namespace()));
        native.set_layer(if shadow { Layer::Top } else { Layer::Overlay });
        native.set_keyboard_mode(KeyboardMode::None);
        native.set_exclusive_zone(-1);
        native.set_anchor(Edge::Top, true);
        native.set_anchor(Edge::Left, true);
        if shadow {
            // GtkWidget сохраняет форму и восстанавливает её при map/resize.
            // Одноразовую форму GdkWindow GTK перезаписывает своей сохранённой.
            native.input_shape_combine_region(Some(&gtk::cairo::Region::create()));
        }
        Ok(())
    })
}

/** Позиция и размер заданы в логических координатах относительно полного монитора. */
pub fn position(
    window: &WebviewWindow,
    monitor: &tauri::Monitor,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> AppResult<()> {
    let scale = monitor.scale_factor();
    let origin = monitor.position();
    let monitor_x = (f64::from(origin.x) / scale).round() as i32;
    let monitor_y = (f64::from(origin.y) / scale).round() as i32;
    let monitor_width = (f64::from(monitor.size().width) / scale).round() as i32;
    let monitor_height = (f64::from(monitor.size().height) / scale).round() as i32;
    with_native(window, move |native| {
        let display = native.display();
        let monitor = (0..display.n_monitors())
            .filter_map(|index| display.monitor(index))
            .find(|monitor| {
                let rect = monitor.geometry();
                rect.x() == monitor_x
                    && rect.y() == monitor_y
                    && rect.width() == monitor_width
                    && rect.height() == monitor_height
            })
            .ok_or_else(|| AppError::from("Overlay target GDK monitor not found"))?;
        native.set_monitor(&monitor);
        native.set_layer_shell_margin(Edge::Left, x);
        native.set_layer_shell_margin(Edge::Top, y);
        native.set_size_request(width as i32, height as i32);
        native.resize(width as i32, height as i32);
        Ok(())
    })
}

/** Отключает своё runtime-правило главного окна при штатном завершении. */
pub fn shutdown() {
    let main = format!("{}_main_window", linux_desktop::namespace());
    let _ = linux_desktop::hyprctl(&[
        "eval",
        &format!("if _G.{main} then _G.{main}:set_enabled(false) end"),
    ]);
}

/** Главное окно получает минимально поддерживаемую раскладкой ширину вне tiling. */
pub fn prepare_main(window_title: &str) -> AppResult<()> {
    let monitors: serde_json::Value =
        serde_json::from_str(&linux_desktop::hyprctl(&["monitors", "-j"])?)?;
    let monitor = monitors.as_array().and_then(|monitors| {
        monitors
            .iter()
            .find(|m| m["focused"].as_bool() == Some(true))
            .or_else(|| monitors.first())
    });
    let (x, y, width, height) = monitor
        .map(|m| {
            let scale = m["scale"].as_f64().unwrap_or(1.0);
            let reserved = |index: usize| m["reserved"][index].as_f64().unwrap_or(0.0);
            let available_width =
                m["width"].as_f64().unwrap_or(1920.0) / scale - reserved(0) - reserved(2);
            let available_height =
                m["height"].as_f64().unwrap_or(1080.0) / scale - reserved(1) - reserved(3);
            let width = available_width.clamp(1.0, 1100.0);
            let height = available_height.clamp(1.0, 700.0);
            (
                (reserved(0) + (available_width - width) / 2.0).round() as i32,
                (reserved(1) + (available_height - height) / 2.0).round() as i32,
                width.round() as u32,
                height.round() as u32,
            )
        })
        .unwrap_or((0, 0, 1100, 700));
    let registry = format!("{}_main_window", linux_desktop::namespace());
    let title_pattern = format!(
        "^{}$",
        window_title.replace(
            ['.', '+', '*', '?', '(', ')', '[', ']', '{', '}', '^', '$', '|', '\\'],
            "."
        )
    );
    let code = format!("if _G.{registry} then _G.{registry}:set_enabled(false) end; _G.{registry}=hl.window_rule({{name={},match={{class='^transcriber$',title={}}},float=true,size={{{width},{height}}},move={{{x},{y}}}}})", quote(&registry), quote(&title_pattern));
    linux_desktop::hyprctl(&["eval", &code])?;
    Ok(())
}

/** Обходит глобальное подавление клиентского maximize, не изменяя правила других окон. */
pub fn toggle_main_maximized(title: &str) -> AppResult<()> {
    let pid = std::process::id();
    let code = format!("for _,w in ipairs(hl.get_windows()) do if w.pid=={pid} and w.title=={} then local r=hl.dispatch(hl.dsp.window.fullscreen({{window=w,mode='maximized',action='toggle',layout_aware=false}})); if not r.ok then error(r.error or 'Hyprland maximize failed') end; return end end; error('Transcriber main window not found')",quote(title));
    linux_desktop::hyprctl(&["eval", &code])?;
    Ok(())
}
