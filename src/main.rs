use eframe::egui;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use web_time::{Duration, Instant};

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 750.0]),
        ..Default::default()
    };
    eframe::run_native(
        "N-Queens Solver (Rust)",
        options,
        Box::new(|cc| {
            // Apply a default style that works well with our theme
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(EightQueensApp::default()))
        }),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` messages to the browser console.
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .and_then(|win| win.document())
            .expect("Could not find document");
        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Could not find canvas")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("Element is not a canvas");

        eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| {
                    cc.egui_ctx.set_visuals(egui::Visuals::dark());
                    // Recommended for web: avoid infinite resize loops
                    cc.egui_ctx.set_pixels_per_point(1.0);
                    Ok(Box::new(EightQueensApp::default()))
                }),
            )
            .await
            .expect("failed to start eframe");
    });
}

// --- Theme Definition ---
#[derive(Clone)]
struct Theme {
    background: egui::Color32,
    panel_background: egui::Color32,
    text_color: egui::Color32,
    accent_color: egui::Color32,
    board_light: egui::Color32,
    board_dark: egui::Color32,
    queen_color: egui::Color32,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            // Sleek Dark Theme
            background: egui::Color32::from_rgb(15, 23, 42), // Slate 900
            panel_background: egui::Color32::from_rgb(30, 41, 59), // Slate 800
            text_color: egui::Color32::from_rgb(226, 232, 240), // Slate 200
            accent_color: egui::Color32::from_rgb(99, 102, 241), // Indigo 500
            board_light: egui::Color32::from_rgb(241, 245, 249), // Slate 100
            board_dark: egui::Color32::from_rgb(100, 116, 139), // Slate 500
            queen_color: egui::Color32::from_rgb(15, 23, 42), // Slate 900
        }
    }
}

// Re-implementing Solver with a distinct "Frame-based" approach
struct EightQueensApp {
    n_input: String,
    n: usize,

    solver: SolverWrapper,

    speed: u64, // 1-10
    paused: bool,
    auto_play: bool,   // True if running continuous
    finding_all: bool, // True if finding all solutions without pausing

    last_update: Instant,
    theme: Theme,
    show_threats: bool,
}

struct SolverWrapper {
    n: usize,
    board: Vec<Vec<u8>>,
    solutions: Vec<String>,

    // DFS State
    stack: Vec<(usize, usize)>,

    // We need to know if we are "forwarding" or "backtracking"
    col: usize,
    row: usize,
    backtracking: bool,

    finished: bool,
    last_solution_board: Option<Vec<Vec<u8>>>,
}

impl SolverWrapper {
    fn new(n: usize) -> Self {
        Self {
            n,
            board: vec![vec![0; n]; n],
            solutions: Vec::new(),
            stack: Vec::new(),
            col: 0,
            row: 0,
            backtracking: false,
            finished: false,
            last_solution_board: None,
        }
    }

    fn step(&mut self) -> bool {
        if self.finished {
            return false;
        }

        if self.backtracking {
            if self.col == 0 && self.row >= self.n {
                self.finished = true;
                return false;
            }

            // Pop previous
            if let Some((r, _)) = self.stack.pop() {
                self.board[r][self.col - 1] = 0; // Remove queen
                self.col -= 1;
                self.row = r + 1; // Try next row
                self.backtracking = false;
            } else {
                self.finished = true;
                return false;
            }
        }

        if self.col >= self.n {
            // Found solution
            self.save_solution();
            self.backtracking = true; // Trigger backtrack to find next
            return true; // Signal solution found
        }

        // Search in current col
        while self.row < self.n {
            if self.is_safe(self.row, self.col) {
                self.board[self.row][self.col] = 1;
                self.stack.push((self.row, self.col));
                self.col += 1;
                self.row = 0;
                return false; // Step complete (placed one queen)
            }
            self.row += 1;
        }

        // No row found in this col, trigger backtrack
        self.backtracking = true;
        false
    }

    fn is_safe(&self, row: usize, col: usize) -> bool {
        for i in 0..col {
            if self.board[row][i] == 1 {
                return false;
            }
        }
        for (i, j) in (0..row).rev().zip((0..col).rev()) {
            if self.board[i][j] == 1 {
                return false;
            }
        }
        for (i, j) in (row + 1..self.n).zip((0..col).rev()) {
            if self.board[i][j] == 1 {
                return false;
            }
        }
        true
    }

    fn save_solution(&mut self) {
        // Store this board state as the last found solution
        self.last_solution_board = Some(self.board.clone());

        let mut parts = Vec::new();
        for c in 0..self.n {
            // Find row
            if let Some(r) = (0..self.n).find(|&r| self.board[r][c] == 1) {
                let file = (b'a' + c as u8) as char;
                let rank = r + 1;
                parts.push(format!("{}{}", file, rank));
            }
        }
        self.solutions.push(parts.join(", "));
    }

    fn restore_last_solution(&mut self) {
        if let Some(board) = &self.last_solution_board {
            self.board = board.clone();
        }
    }
}

impl Default for EightQueensApp {
    fn default() -> Self {
        Self {
            n_input: "8".to_owned(),
            n: 8,
            solver: SolverWrapper::new(8),
            speed: 5,
            paused: true,
            auto_play: false,
            finding_all: false,
            last_update: Instant::now(),
            theme: Theme::default(),
            show_threats: false,
        }
    }
}

impl eframe::App for EightQueensApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- Update Logic ---
        let delay_ms = if self.speed == 10 {
            0
        } else {
            (10 - self.speed) * 50
        };

        if self.auto_play && !self.solver.finished {
            if self.speed == 10 {
                let start = Instant::now();
                while start.elapsed() < Duration::from_millis(16) && !self.solver.finished {
                    if self.solver.step() {
                        if !self.finding_all {
                            self.paused = true;
                            self.auto_play = false;
                            break;
                        }
                    }
                }
                if self.solver.finished {
                    self.solver.restore_last_solution();
                }
                ctx.request_repaint();
            } else {
                if self.last_update.elapsed().as_millis() as u64 >= delay_ms {
                    if self.solver.step() {
                        if !self.finding_all {
                            self.paused = true;
                            self.auto_play = false;
                        }
                    }
                    self.last_update = Instant::now();
                }
                ctx.request_repaint();
            }
        } else if !self.paused && !self.solver.finished {
            if self.last_update.elapsed().as_millis() as u64 >= delay_ms {
                if self.solver.step() {
                    if !self.finding_all {
                        self.paused = true;
                    }
                }
                self.last_update = Instant::now();
            }
            ctx.request_repaint();
        }

        // --- Custom Styles ---
        let mut style = (*ctx.style()).clone();
        style.visuals.widgets.noninteractive.bg_fill = self.theme.background;
        style.visuals.window_fill = self.theme.background;
        style.visuals.selection.bg_fill = self.theme.accent_color;

        // Define a custom frame for panels
        let panel_frame = egui::Frame::none()
            .fill(self.theme.panel_background)
            .inner_margin(12.0)
            .rounding(10.0)
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_white_alpha(20)));

        // --- GUI ---

        // 1. Right Side Control Panel
        egui::SidePanel::right("controls")
            .frame(panel_frame)
            .min_width(320.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.add_space(8.0);

                // Header
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("♛ N-Queens")
                            .size(24.0)
                            .strong()
                            .color(self.theme.text_color),
                    );
                });
                ui.add_space(20.0);

                // Config Section
                ui.label(
                    egui::RichText::new("Configuration")
                        .strong()
                        .color(self.theme.text_color),
                );
                ui.separator();

                egui::Grid::new("config_grid")
                    .num_columns(2)
                    .spacing([10.0, 10.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Board Size:").color(self.theme.text_color));
                        let response = ui
                            .add(egui::TextEdit::singleline(&mut self.n_input).desired_width(50.0));
                        if response.lost_focus()
                            && response.ctx.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            // Optional: trigger restart on enter?
                        }
                        ui.end_row();
                    });

                ui.add_space(15.0);

                ui.label(
                    egui::RichText::new("Controls")
                        .strong()
                        .color(self.theme.text_color),
                );
                ui.separator();

                ui.horizontal(|ui| {
                    let btn_size = egui::vec2(50.0, 40.0);
                    let font_size = 20.0;

                    // Play (▶) - Merged Start and Resume
                    let play_icon = egui::RichText::new("▶").size(font_size);
                    if ui
                        .add_sized(btn_size, egui::Button::new(play_icon))
                        .on_hover_text("Start or resume the visual search")
                        .clicked()
                    {
                        if self.solver.finished {
                            self.solver = SolverWrapper::new(self.n);
                        }

                        if let Ok(n) = self.n_input.parse::<usize>() {
                            if n != self.n {
                                // New game
                                self.n = n;
                                self.solver = SolverWrapper::new(self.n);
                            }
                        }

                        self.paused = false;
                        self.auto_play = false;
                        self.finding_all = false;
                    }

                    // Step (|▶)
                    let step_icon = egui::RichText::new("|▶").size(font_size);

                    if ui
                        .add_sized(btn_size, egui::Button::new(step_icon))
                        .on_hover_text("Execute a single step in the search")
                        .clicked()
                    {
                        self.solver.step();
                        self.paused = true;
                        self.auto_play = false;
                        self.finding_all = false;
                    }

                    // Fast Forward (⏩) - Merged Quick Run and Next Solution
                    // Logic: If not finished, find next solution instantly.
                    let ff_icon = egui::RichText::new("⏩").size(font_size);
                    if ui
                        .add_sized(btn_size, egui::Button::new(ff_icon))
                        .on_hover_text("Skip to the next solution")
                        .clicked()
                    {
                        if self.solver.finished {
                        } else {
                            // Find next solution
                            while !self.solver.finished {
                                if self.solver.step() {
                                    break;
                                }
                            }
                            self.paused = true; // Pause on solution
                            self.finding_all = false;
                            self.solver.backtracking = true; // Prep for next
                        }
                    }

                    // Find All (⏭) - Next Track Icon
                    // Logic: Auto-play until done.
                    let next_track_icon = egui::RichText::new("⏭").size(font_size);
                    if ui
                        .add_sized(btn_size, egui::Button::new(next_track_icon))
                        .on_hover_text("Find all solutions at maximum speed")
                        .clicked()
                    {
                        if self.solver.finished {
                            self.solver = SolverWrapper::new(self.n);
                        }

                        if let Ok(n) = self.n_input.parse::<usize>() {
                            if n != self.n {
                                self.n = n;
                                self.solver = SolverWrapper::new(self.n);
                            }
                        }

                        self.auto_play = true;
                        self.finding_all = true;
                        self.speed = 10;
                        self.paused = false;
                    }

                    // Stop / Restart (◼)
                    // Logic: If running/not empty, first click pauses (or stops).
                    // Let's implement user request: "Stop/Restart key ... capable to stop running"
                    // If we treat it as "reset", that's easy.
                    // If we treat it as "stop", it just sets paused=true.
                    // Let's do: If !paused -> Pause. If paused -> Restart (Reset).
                    // Actually, standard media stop usually resets.
                    // Let's play it safe: Click -> Stop (Pause) + Reset Board.
                    // "Restart key change to Stop/Restart key (need to change its function and make it capable to stop running). Use stop Icon for It."
                    // Implies: If running, STOP. If stopped, RESTART.

                    let stop_icon = egui::RichText::new("◼").size(font_size);
                    if ui
                        .add_sized(btn_size, egui::Button::new(stop_icon))
                        .on_hover_text("Stop running or reset the board")
                        .clicked()
                    {
                        if !self.paused && !self.solver.finished {
                            // Currently running -> Stop (Pause)
                            self.paused = true;
                            self.auto_play = false;
                            self.finding_all = false;
                        } else {
                            // Already stopped or finished -> Restart
                            if let Ok(n) = self.n_input.parse::<usize>() {
                                if n >= 4 {
                                    self.n = n;
                                }
                            }
                            self.solver = SolverWrapper::new(self.n);
                            self.paused = true;
                            self.auto_play = false;
                            self.finding_all = false;
                        }
                    }
                });

                ui.add_space(10.0);
                ui.label(egui::RichText::new("Speed").color(self.theme.text_color));
                ui.add(egui::Slider::new(&mut self.speed, 1..=10).show_value(false));

                ui.add_space(8.0);
                ui.checkbox(&mut self.show_threats, "Show Threatened Squares");

                ui.add_space(20.0);

                // Stats
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        egui::RichText::new(format!(
                            "Solutions Found: {}",
                            self.solver.solutions.len()
                        ))
                        .strong()
                        .size(16.0),
                    );
                });

                ui.add_space(10.0);
                ui.add_space(10.0);
                #[cfg(not(target_arch = "wasm32"))]
                if ui.button("Export to CSV").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("CSV", &["csv"])
                        .save_file()
                    {
                        if let Ok(mut wtr) =
                            csv::WriterBuilder::new().delimiter(b'\t').from_path(path)
                        {
                            // Headers: Solution No., a, b, c, ...
                            let mut headers = vec!["Solution No.".to_string()];
                            for i in 0..self.n {
                                headers.push(((b'a' + i as u8) as char).to_string());
                            }
                            let _ = wtr.write_record(&headers);

                            for (i, sol) in self.solver.solutions.iter().enumerate() {
                                let mut row = vec![(i + 1).to_string()];
                                let parts: Vec<&str> = sol.split(", ").collect();
                                for part in parts {
                                    let rank = &part[1..];
                                    row.push(rank.to_string());
                                }
                                let _ = wtr.write_record(&row);
                            }
                        }
                    }
                }
                #[cfg(target_arch = "wasm32")]
                if ui.button("Export to CSV (Native only)").clicked() {
                    // Export is trickier on web, disabling for now.
                }

                ui.add_space(20.0);

                ui.label(
                    egui::RichText::new("Solutions History")
                        .strong()
                        .color(self.theme.text_color),
                );
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for (i, sol) in self.solver.solutions.iter().enumerate() {
                            ui.label(
                                egui::RichText::new(format!("#{}: {}", i + 1, sol))
                                    .monospace()
                                    .size(12.0),
                            );
                        }
                    });
            });

        // 2. Central Panel (Board)
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(self.theme.background))
            .show(ctx, |ui| {
                let available_rect = ui.available_rect_before_wrap();
                let size = available_rect.height().min(available_rect.width()) - 60.0;
                let center = available_rect.center();

                let board_rect = egui::Rect::from_center_size(center, egui::vec2(size, size));

                // Draw background shadow/border
                ui.painter().rect_filled(
                    board_rect.expand(5.0),
                    5.0,
                    self.theme.text_color.linear_multiply(0.2), // Subtle shadow
                );

                let cell_size = size / self.n as f32;
                let painter = ui.painter();

                // Draw Board
                for row in 0..self.n {
                    for col in 0..self.n {
                        let x = board_rect.min.x + col as f32 * cell_size;
                        let y = board_rect.min.y + row as f32 * cell_size;
                        let cell_rect = egui::Rect::from_min_size(
                            egui::pos2(x, y),
                            egui::vec2(cell_size, cell_size),
                        );

                        let color = if (row + col) % 2 == 0 {
                            self.theme.board_light
                        } else {
                            self.theme.board_dark
                        };

                        painter.rect_filled(cell_rect, 0.0, color);

                        if self.show_threats {
                            // Logic: highlight if share row, col, or diag with ANY queen
                            let mut threatened = false;
                            for r in 0..self.n {
                                for c in 0..self.n {
                                    if self.solver.board[r][c] == 1 {
                                        // Ignore current square being queen itself for threat?
                                        // Usually threatened means where you can't place.
                                        if r == row
                                            || c == col
                                            || (r as i32 - row as i32).abs()
                                                == (c as i32 - col as i32).abs()
                                        {
                                            if r != row || c != col {
                                                threatened = true;
                                                break;
                                            }
                                        }
                                    }
                                }
                                if threatened {
                                    break;
                                }
                            }

                            if threatened {
                                painter.rect_filled(
                                    cell_rect.shrink(2.0),
                                    2.0,
                                    egui::Color32::from_rgba_unmultiplied(255, 0, 0, 40),
                                );
                            }
                        }

                        // Highlight placement (optional, simple check)
                        if self.solver.board[row][col] == 1 {
                            // Draw Queen with better styling
                            // Circle background
                            let center = cell_rect.center();
                            // Text default "♛"
                            let font_size = cell_size * 0.7;

                            // We could do a unicode shadow
                            // painter.text(center + egui::vec2(2.0, 2.0), egui::Align2::CENTER_CENTER, "♛", egui::FontId::proportional(font_size), egui::Color32::BLACK.linear_multiply(0.3));

                            painter.text(
                                center,
                                egui::Align2::CENTER_CENTER,
                                "♛",
                                egui::FontId::proportional(font_size),
                                self.theme.queen_color,
                            );
                        }
                    }
                }

                // Draw Coordinates
                for i in 0..self.n {
                    let font_id = egui::FontId::proportional(cell_size * 0.15);
                    let col_char = (b'a' + i as u8) as char;
                    let row_char = (i + 1).to_string();

                    // Files (bottom)
                    let x = board_rect.min.x + i as f32 * cell_size + cell_size / 2.0;
                    let y = board_rect.max.y + 10.0;
                    painter.text(
                        egui::pos2(x, y),
                        egui::Align2::CENTER_TOP,
                        col_char.to_string(),
                        font_id.clone(),
                        self.theme.text_color,
                    );

                    // Ranks (left)
                    let x = board_rect.min.x - 10.0;
                    let y = board_rect.min.y + i as f32 * cell_size + cell_size / 2.0;
                    painter.text(
                        egui::pos2(x, y),
                        egui::Align2::RIGHT_CENTER,
                        row_char,
                        font_id.clone(),
                        self.theme.text_color,
                    );
                }
            });
    }
}
