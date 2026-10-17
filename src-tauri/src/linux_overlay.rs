//! Hyprland управляет геометрией Wayland-окон вместо запрещённого set_position.

use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

use crate::{error::AppResult, linux_desktop};

type Geometry = (i32, i32, u32, u32);
static GEOMETRY: OnceLock<Mutex<HashMap<String, Geometry>>> = OnceLock::new();

fn quote(value: &str) -> String {
    serde_json::to_string(value).expect("string is serializable")
}

/** Уникальный initial title позволяет правилам затрагивать только собственное окно. */
pub fn title(label: &str) -> String {
    format!("{}_{label}", linux_desktop::namespace())
}

/** Регистрируется до построения GTK-окна, чтобы оно не захватило фокус при первом map. */
pub fn prepare(label: &str) -> AppResult<()> {
    apply(label, "")
}

fn apply(label: &str, geometry: &str) -> AppResult<()> {
    let registry = format!("{}_overlays", linux_desktop::namespace());
    let id = quote(label);
    let spec = format!("{{name={},match={{title={}}},float=true,pin=true,no_initial_focus=true,no_focus=true,no_follow_mouse=true,focus_on_activate=false,no_anim=true,no_blur=true,no_shadow=true,decorate=false,border_size=0,rounding=0,{geometry}}}", quote(&title(label)), quote(&format!("^{}$", title(label))));
    let code = format!("local r=_G.{registry}; if not r then r={{}}; _G.{registry}=r end; if not r[{id}] or r[{id}].spec~={} then if r[{id}] then r[{id}].rule:set_enabled(false) end; r[{id}]={{spec={},rule=hl.window_rule({spec})}} end; r[{id}].rule:set_enabled(true)", quote(&spec), quote(&spec));
    linux_desktop::hyprctl(&["eval", &code])?;
    Ok(())
}

/** Геометрия задана в логических координатах относительно указанного монитора. */
pub fn position(
    label: &str,
    monitor: Option<&str>,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> AppResult<()> {
    let origin = linux_desktop::hyprctl(&["monitors", "-j"])
        .ok()
        .and_then(|data| serde_json::from_str::<serde_json::Value>(&data).ok())
        .and_then(|data| {
            data.as_array().and_then(|monitors| {
                monitors
                    .iter()
                    .find(|entry| entry["name"].as_str() == monitor)
                    .cloned()
            })
        })
        .map(|entry| {
            (
                entry["x"].as_i64().unwrap_or(0) as i32,
                entry["y"].as_i64().unwrap_or(0) as i32,
            )
        })
        .unwrap_or((0, 0));
    if let Ok(mut positions) = GEOMETRY.get_or_init(|| Mutex::new(HashMap::new())).lock() {
        positions.insert(
            label.to_owned(),
            (origin.0 + x, origin.1 + y, width, height),
        );
    }
    let monitor = monitor
        .map(|name| format!("monitor={},", quote(name)))
        .unwrap_or_default();
    apply(
        label,
        &format!("{monitor}move={{{x},{y}}},size={{{width},{height}}},"),
    )
}

/** Обновляет уже отображённое окно адресно, не меняя фокус целевого приложения. */
pub fn place_visible(label: &str) -> AppResult<()> {
    let position = GEOMETRY
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .ok()
        .and_then(|positions| positions.get(label).copied());
    let Some((x, y, width, height)) = position else {
        return Ok(());
    };
    let title = quote(&title(label));
    let code = format!("local function dispatch(d) local r=hl.dispatch(d); if not r.ok then error(r.error or 'Hyprland dispatch failed') end end; for _,w in ipairs(hl.get_windows()) do if w.title=={title} then dispatch(hl.dsp.window.float({{window=w,action='enable'}})); dispatch(hl.dsp.window.pin({{window=w,action='enable'}})); dispatch(hl.dsp.window.resize({{window=w,x={width},y={height},relative=false}})); dispatch(hl.dsp.window.move({{window=w,x={x},y={y},relative=false}})); dispatch(hl.dsp.window.alter_zorder({{window=w,mode='top'}})) end end");
    linux_desktop::hyprctl(&["eval", &code])?;
    Ok(())
}

/** Отключает свои runtime-правила при штатном завершении приложения. */
pub fn shutdown() {
    let main = format!("{}_main_window", linux_desktop::namespace());
    let _ = linux_desktop::hyprctl(&[
        "eval",
        &format!("if _G.{main} then _G.{main}:set_enabled(false) end"),
    ]);
    let registry = format!("{}_overlays", linux_desktop::namespace());
    let _ = linux_desktop::hyprctl(&["eval", &format!("local r=_G.{registry}; if r then for _,v in pairs(r) do v.rule:set_enabled(false) end end")]);
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
