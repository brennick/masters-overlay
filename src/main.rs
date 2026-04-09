#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const API_URL: &str = "https://www.masters.com/en_US/scores/feeds/2026/scores.json";
const REFRESH_SECS: u64 = 60;

// Masters green palette
const MASTERS_GREEN: egui::Color32 = egui::Color32::from_rgb(0, 98, 65);
const DARK_GREEN: egui::Color32 = egui::Color32::from_rgb(0, 70, 46);
const GOLD: egui::Color32 = egui::Color32::from_rgb(198, 163, 68);
const TEXT_WHITE: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);
const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(180, 200, 180);
const RED: egui::Color32 = egui::Color32::from_rgb(220, 80, 80);
const ROW_ALT: egui::Color32 = egui::Color32::from_rgb(0, 82, 54);

#[derive(Debug, Clone, Deserialize)]
struct ScoresResponse {
    data: Option<ScoresData>,
}

#[derive(Debug, Clone, Deserialize)]
struct ScoresData {
    player: Option<Vec<PlayerData>>,
}

#[derive(Debug, Clone, Deserialize)]
struct PlayerData {
    pos: Option<String>,
    #[serde(alias = "first_name")]
    #[serde(alias = "firstName")]
    first_name: Option<String>,
    #[serde(alias = "last_name")]
    #[serde(alias = "lastName")]
    last_name: Option<String>,
    #[serde(alias = "display_name")]
    #[serde(alias = "displayName")]
    display_name: Option<String>,
    #[serde(alias = "display_name2")]
    display_name2: Option<String>,
    topar: Option<serde_json::Value>,
    #[serde(alias = "toPar")]
    to_par: Option<serde_json::Value>,
    today: Option<serde_json::Value>,
    thru: Option<serde_json::Value>,
    #[serde(alias = "total")]
    total: Option<serde_json::Value>,
    status: Option<String>,
    // Round scores
    round1: Option<serde_json::Value>,
    round2: Option<serde_json::Value>,
    round3: Option<serde_json::Value>,
    round4: Option<serde_json::Value>,
}

impl PlayerData {
    fn name(&self) -> String {
        if let Some(ref dn) = self.display_name2 {
            if !dn.is_empty() {
                return dn.clone();
            }
        }
        if let Some(ref dn) = self.display_name {
            if !dn.is_empty() {
                return dn.clone();
            }
        }
        let first = self.first_name.as_deref().unwrap_or("");
        let last = self.last_name.as_deref().unwrap_or("Unknown");
        if first.is_empty() {
            last.to_string()
        } else {
            format!("{} {}", &first[..1], last)
        }
    }

    fn to_par_str(&self) -> String {
        self.to_par
            .as_ref()
            .or(self.topar.as_ref())
            .map(val_to_string)
            .unwrap_or_else(|| "-".into())
    }

    fn today_str(&self) -> String {
        self.today.as_ref().map(val_to_string).unwrap_or_else(|| "-".into())
    }

    fn thru_str(&self) -> String {
        self.thru.as_ref().map(val_to_string).unwrap_or_else(|| "-".into())
    }

    fn total_str(&self) -> String {
        self.total.as_ref().map(val_to_string).unwrap_or_else(|| "-".into())
    }

    fn pos_str(&self) -> String {
        self.pos.clone().unwrap_or_else(|| "-".into())
    }
}

fn val_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => v.to_string(),
    }
}

fn score_color(s: &str) -> egui::Color32 {
    let trimmed = s.trim();
    if trimmed.starts_with('-') {
        RED
    } else if trimmed == "E" || trimmed == "0" {
        TEXT_DIM
    } else if trimmed.starts_with('+') || trimmed.parse::<i32>().map_or(false, |n| n > 0) {
        egui::Color32::from_rgb(130, 200, 130)
    } else {
        TEXT_WHITE
    }
}

#[derive(Clone)]
struct FetchState {
    players: Vec<PlayerData>,
    error: Option<String>,
    last_refresh: Option<Instant>,
    raw_json_keys: Option<String>,
}

impl Default for FetchState {
    fn default() -> Self {
        Self {
            players: Vec::new(),
            error: None,
            last_refresh: None,
            raw_json_keys: None,
        }
    }
}

fn fetch_scores(state: &Arc<Mutex<FetchState>>) {
    let state = Arc::clone(state);
    std::thread::spawn(move || {
        let result = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("MastersOverlay/1.0")
            .build()
            .and_then(|c| c.get(API_URL).send())
            .and_then(|r| r.text());

        let mut st = state.lock().unwrap();
        match result {
            Ok(body) => {
                // Try parsing as structured response first
                if let Ok(resp) = serde_json::from_str::<ScoresResponse>(&body) {
                    if let Some(data) = resp.data {
                        if let Some(players) = data.player {
                            st.players = players;
                            st.error = None;
                            st.last_refresh = Some(Instant::now());
                            return;
                        }
                    }
                }

                // Fallback: try to find a player array anywhere in the JSON
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body) {
                    // Save top-level keys for debugging
                    if let Some(obj) = val.as_object() {
                        let keys: Vec<&String> = obj.keys().collect();
                        st.raw_json_keys = Some(format!("Top keys: {:?}", keys));
                    }

                    // Search for player array in common locations
                    let candidates = [
                        &val["data"]["player"],
                        &val["data"]["players"],
                        &val["players"],
                        &val["player"],
                        &val["leaderboard"],
                        &val["data"]["leaderboard"],
                        &val["results"],
                        &val["data"]["results"],
                    ];

                    for candidate in &candidates {
                        if let Ok(players) = serde_json::from_value::<Vec<PlayerData>>((*candidate).clone()) {
                            if !players.is_empty() {
                                st.players = players;
                                st.error = None;
                                st.last_refresh = Some(Instant::now());
                                return;
                            }
                        }
                    }

                    st.error = Some(format!(
                        "Could not find player data. {}",
                        st.raw_json_keys.clone().unwrap_or_default()
                    ));
                } else {
                    st.error = Some("Invalid JSON response".into());
                }
            }
            Err(e) => {
                st.error = Some(format!("Fetch error: {}", e));
            }
        }
        st.last_refresh = Some(Instant::now());
    });
}

struct MastersApp {
    state: Arc<Mutex<FetchState>>,
    last_fetch: Instant,
}

impl MastersApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Style setup
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals.window_fill = DARK_GREEN;
        style.visuals.panel_fill = DARK_GREEN;
        style.visuals.override_text_color = Some(TEXT_WHITE);
        cc.egui_ctx.set_style(style);

        let state = Arc::new(Mutex::new(FetchState::default()));
        fetch_scores(&state);

        Self {
            state,
            last_fetch: Instant::now(),
        }
    }
}

impl eframe::App for MastersApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Auto-refresh every 60 seconds
        if self.last_fetch.elapsed() >= Duration::from_secs(REFRESH_SECS) {
            fetch_scores(&self.state);
            self.last_fetch = Instant::now();
        }

        // Request repaint every second to keep UI responsive
        ctx.request_repaint_after(Duration::from_secs(1));

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(DARK_GREEN).inner_margin(0.0))
            .show(ctx, |ui| {
                // Title bar area
                let full_w = ui.available_width();
                let title_rect = ui.allocate_space(egui::vec2(full_w, 36.0)).1;
                ui.painter().rect_filled(title_rect, 0.0, MASTERS_GREEN);
                ui.painter().text(
                    title_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "THE MASTERS",
                    egui::FontId::proportional(18.0),
                    GOLD,
                );

                // Split title bar into drag zone (left) and close button (right)
                let close_size = 36.0;
                let close_rect = egui::Rect::from_min_size(
                    egui::pos2(title_rect.right() - close_size, title_rect.top()),
                    egui::vec2(close_size, 36.0),
                );
                let drag_rect = egui::Rect::from_min_max(
                    title_rect.left_top(),
                    egui::pos2(title_rect.right() - close_size, title_rect.bottom()),
                );

                // Close button — painted as "X" lines
                let close_resp = ui.allocate_rect(close_rect, egui::Sense::click());
                let close_color = if close_resp.hovered() { RED } else { TEXT_DIM };
                let cc = close_rect.center();
                let s = 6.0; // half-size of X
                let stroke = egui::Stroke::new(2.0, close_color);
                ui.painter().line_segment([egui::pos2(cc.x - s, cc.y - s), egui::pos2(cc.x + s, cc.y + s)], stroke);
                ui.painter().line_segment([egui::pos2(cc.x + s, cc.y - s), egui::pos2(cc.x - s, cc.y + s)], stroke);
                if close_resp.clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }

                // Drag zone — delegate to OS
                let title_resp = ui.allocate_rect(drag_rect, egui::Sense::drag());
                if title_resp.drag_started() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }

                // Column positions (shared between header and rows)
                let w = ui.available_width();
                let left = ui.cursor().left() + 4.0;
                let col_pos_x = left;
                let col_name_x = col_pos_x + 34.0;
                let col_par_x = col_name_x + (w * 0.38);
                let col_today_x = col_par_x + 52.0;
                let col_thru_x = col_today_x + 52.0;

                // Leaderboard header
                ui.add_space(2.0);
                {
                    let (_, rect) = ui.allocate_space(egui::vec2(w, 20.0));
                    let y = rect.center().y;
                    let hfont = egui::FontId::proportional(14.0);
                    ui.painter().text(egui::pos2(col_pos_x, y), egui::Align2::LEFT_CENTER, "POS", hfont.clone(), GOLD);
                    ui.painter().text(egui::pos2(col_name_x, y), egui::Align2::LEFT_CENTER, "PLAYER", hfont.clone(), GOLD);
                    ui.painter().text(egui::pos2(col_par_x + 26.0, y), egui::Align2::CENTER_CENTER, "PAR", hfont.clone(), GOLD);
                    ui.painter().text(egui::pos2(col_today_x + 26.0, y), egui::Align2::CENTER_CENTER, "TODAY", hfont.clone(), GOLD);
                    ui.painter().text(egui::pos2(col_thru_x + 26.0, y), egui::Align2::CENTER_CENTER, "THRU", hfont.clone(), GOLD);
                }

                ui.painter().hline(
                    ui.cursor().x_range(),
                    ui.cursor().top(),
                    egui::Stroke::new(1.0, GOLD),
                );
                ui.add_space(2.0);

                // Player rows
                let state = self.state.lock().unwrap().clone();

                if let Some(ref err) = state.error {
                    ui.add_space(20.0);
                    ui.vertical_centered(|ui| {
                        ui.colored_label(RED, egui::RichText::new(err).font(egui::FontId::proportional(11.0)));
                    });
                    if state.players.is_empty() {
                        return;
                    }
                    ui.add_space(10.0);
                }

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let font = egui::FontId::proportional(15.0);
                    let row_h = 26.0;

                    for (i, player) in state.players.iter().enumerate() {
                        let bg = if i % 2 == 0 { DARK_GREEN } else { ROW_ALT };
                        let (_, rect) = ui.allocate_space(egui::vec2(w, row_h));
                        ui.painter().rect_filled(rect, 0.0, bg);
                        let y = rect.center().y;

                        // Position
                        ui.painter().text(
                            egui::pos2(col_pos_x, y), egui::Align2::LEFT_CENTER,
                            player.pos_str(), font.clone(), TEXT_WHITE,
                        );

                        // Name (truncate to fit)
                        let name = player.name();
                        let name_width = col_par_x - col_name_x - 4.0;
                        let max_chars = (name_width / 7.0) as usize;
                        let display_name = if name.len() > max_chars {
                            format!("{}...", &name[..max_chars.saturating_sub(3)])
                        } else {
                            name
                        };
                        ui.painter().text(
                            egui::pos2(col_name_x, y), egui::Align2::LEFT_CENTER,
                            display_name, font.clone(), TEXT_WHITE,
                        );

                        // To par
                        let par_str = player.to_par_str();
                        ui.painter().text(
                            egui::pos2(col_par_x + 26.0, y), egui::Align2::CENTER_CENTER,
                            &par_str, font.clone(), score_color(&par_str),
                        );

                        // Today
                        let today_str = player.today_str();
                        ui.painter().text(
                            egui::pos2(col_today_x + 26.0, y), egui::Align2::CENTER_CENTER,
                            &today_str, font.clone(), score_color(&today_str),
                        );

                        // Thru
                        ui.painter().text(
                            egui::pos2(col_thru_x + 26.0, y), egui::Align2::CENTER_CENTER,
                            player.thru_str(), font.clone(), TEXT_DIM,
                        );
                    }
                });

                // Last refresh indicator + resize handle at bottom
                let max = ui.max_rect();
                if let Some(last) = state.last_refresh {
                    let elapsed = last.elapsed().as_secs();
                    ui.painter().text(
                        egui::pos2(max.right() - 6.0, max.bottom() - 18.0),
                        egui::Align2::RIGHT_CENTER,
                        format!("{}s ago", elapsed),
                        egui::FontId::proportional(9.0),
                        TEXT_DIM,
                    );
                }

                // Bottom resize handle (6px tall bar at the very bottom)
                let handle_rect = egui::Rect::from_min_max(
                    egui::pos2(max.left(), max.bottom() - 6.0),
                    max.right_bottom(),
                );
                let handle_resp = ui.allocate_rect(handle_rect, egui::Sense::drag());

                // Draw grip dots
                let grip_color = if handle_resp.hovered() || handle_resp.dragged() { GOLD } else { TEXT_DIM };
                let cy = handle_rect.center().y;
                let cx = handle_rect.center().x;
                for dx in [-8.0, 0.0, 8.0] {
                    ui.painter().circle_filled(egui::pos2(cx + dx, cy), 1.5, grip_color);
                }

                // Change cursor on hover
                if handle_resp.hovered() || handle_resp.dragged() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                }

                if handle_resp.dragged() {
                    let delta_y = handle_resp.drag_delta().y;
                    if let Some(inner) = ctx.input(|i| i.viewport().inner_rect) {
                        let new_h = (inner.height() + delta_y).max(200.0);
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                            egui::vec2(inner.width(), new_h),
                        ));
                    }
                }
            });
    }
}

fn main() -> eframe::Result<()> {
    let (screen_w, screen_h) = get_screen_size().unwrap_or((1920.0, 1080.0));
    let win_width = (screen_w / 8.0).max(200.0);
    let win_height = screen_h * 0.85;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([win_width, win_height])
            .with_position([screen_w - win_width - 8.0, 40.0])
            .with_always_on_top()
            .with_decorations(false)
            .with_transparent(false)
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "Masters Overlay",
        options,
        Box::new(|cc| Ok(Box::new(MastersApp::new(cc)))),
    )
}

#[cfg(target_os = "windows")]
fn get_screen_size() -> Option<(f32, f32)> {
    use std::process::Command;
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Add-Type -AssemblyName System.Windows.Forms; $s = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds; \"$($s.Width) $($s.Height)\"",
        ])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = text.trim().split_whitespace().collect();
    if parts.len() == 2 {
        Some((parts[0].parse().ok()?, parts[1].parse().ok()?))
    } else {
        None
    }
}

#[cfg(not(target_os = "windows"))]
fn get_screen_size() -> Option<(f32, f32)> {
    None
}

