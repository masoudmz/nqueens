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

#[derive(Clone, PartialEq)]
struct Theme {
    name: &'static str,
    background: egui::Color32,
    panel_background: egui::Color32,
    text_color: egui::Color32,
    accent_color: egui::Color32,
    board_light: egui::Color32,
    board_dark: egui::Color32,
    queen_color: egui::Color32,
}

impl Theme {
    fn presets() -> Vec<Self> {
        vec![
            Self {
                name: "Sleek Dark",
                background: egui::Color32::from_rgb(15, 23, 42),
                panel_background: egui::Color32::from_rgb(30, 41, 59),
                text_color: egui::Color32::from_rgb(226, 232, 240),
                accent_color: egui::Color32::from_rgb(99, 102, 241),
                board_light: egui::Color32::from_rgb(241, 245, 249),
                board_dark: egui::Color32::from_rgb(100, 116, 139),
                queen_color: egui::Color32::from_rgb(15, 23, 42),
            },
            Self {
                name: "Classic Wood",
                background: egui::Color32::from_rgb(45, 25, 10),
                panel_background: egui::Color32::from_rgb(70, 40, 20),
                text_color: egui::Color32::from_rgb(245, 230, 200),
                accent_color: egui::Color32::from_rgb(180, 100, 40),
                board_light: egui::Color32::from_rgb(210, 180, 140),
                board_dark: egui::Color32::from_rgb(139, 69, 19),
                queen_color: egui::Color32::from_rgb(45, 25, 10),
            },
            Self {
                name: "Neon Night",
                background: egui::Color32::from_rgb(10, 10, 20),
                panel_background: egui::Color32::from_rgb(20, 20, 40),
                text_color: egui::Color32::from_rgb(0, 255, 255),
                accent_color: egui::Color32::from_rgb(255, 0, 255),
                board_light: egui::Color32::from_rgb(30, 30, 60),
                board_dark: egui::Color32::from_rgb(15, 15, 30),
                queen_color: egui::Color32::from_rgb(255, 255, 0),
            },
            Self {
                name: "Paper",
                background: egui::Color32::from_rgb(240, 240, 230),
                panel_background: egui::Color32::from_rgb(220, 220, 210),
                text_color: egui::Color32::from_rgb(50, 50, 50),
                accent_color: egui::Color32::from_rgb(200, 50, 50),
                board_light: egui::Color32::from_rgb(255, 255, 250),
                board_dark: egui::Color32::from_rgb(200, 200, 190),
                queen_color: egui::Color32::from_rgb(20, 20, 20),
            },
        ]
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::presets()[0].clone()
    }
}

struct Particle {
    pos: egui::Pos2,
    vel: egui::Vec2,
    color: egui::Color32,
    life: f32, // 1.0 down to 0.0
    size: f32,
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
    only_unique: bool,
    particles: Vec<Particle>,
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
    unique_solutions: Vec<Vec<usize>>, // Store row indices
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
            unique_solutions: Vec::new(),
        }
    }

    fn get_variants(sol: &[usize]) -> Vec<Vec<usize>> {
        let n = sol.len();
        let mut variants = Vec::new();

        // 1. Convert to (x, y) coordinates
        let coords: Vec<(usize, usize)> = sol.iter().enumerate().map(|(x, &y)| (x, y)).collect();

        // Helper to convert back to sol vector
        let to_sol = |pts: &[(usize, usize)]| -> Vec<usize> {
            let mut v = vec![0; n];
            for &(x, y) in pts {
                v[x] = y;
            }
            v
        };

        // All 8 transformations
        // (x, y) ->
        // 1. (x, y)
        // 2. (y, n-1-x) - rotate 90
        // 3. (n-1-x, n-1-y) - rotate 180
        // 4. (n-1-y, x) - rotate 270
        // 5. (n-1-x, y) - flip H
        // 6. (x, n-1-y) - flip V
        // 7. (y, x) - flip D1
        // 8. (n-1-y, n-1-x) - flip D2

        let mut curr = coords.clone();
        for _ in 0..4 {
            // Rotate
            variants.push(to_sol(&curr));
            // Flip H
            let flipped: Vec<(usize, usize)> = curr.iter().map(|&(x, y)| (n - 1 - x, y)).collect();
            variants.push(to_sol(&flipped));

            // Apply 90 rotation for next iteration
            curr = curr.iter().map(|&(x, y)| (y, n - 1 - x)).collect();
        }

        variants
    }

    fn is_new_unique(&self, sol: &[usize]) -> bool {
        let variants = Self::get_variants(sol);
        for v in variants {
            if self.unique_solutions.contains(&v) {
                return false;
            }
        }
        true
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
        let mut queen_rows = vec![0; self.n];
        let mut parts = Vec::new();
        for c in 0..self.n {
            if let Some(r) = (0..self.n).find(|&r| self.board[r][c] == 1) {
                queen_rows[c] = r;
                let file = (b'a' + c as u8) as char;
                let rank = r + 1;
                parts.push(format!("{}{}", file, rank));
            }
        }

        let sol_str = parts.join(", ");
        if !self.is_new_unique(&queen_rows) {
            // Already seen a variant of this
            self.last_solution_board = Some(self.board.clone());
            self.unique_solutions.push(queen_rows); // We still store it to mark as non-unique if needed, but usually we just want the list of strings
            self.solutions.push(format!("(Sym) {}", sol_str));
        } else {
            self.last_solution_board = Some(self.board.clone());
            self.unique_solutions.push(queen_rows);
            self.solutions.push(sol_str);
        }
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
            only_unique: false,
            particles: Vec::new(),
        }
    }
}

impl EightQueensApp {
    fn spawn_particles(&mut self, pos: egui::Pos2, color: egui::Color32) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        for _ in 0..30 {
            let angle: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
            let speed: f32 = rng.gen_range(100.0..500.0);
            self.particles.push(Particle {
                pos,
                vel: egui::vec2(angle.cos() * speed, angle.sin() * speed - 200.0),
                color,
                life: 1.0,
                size: rng.gen_range(3.0..7.0),
            });
        }
    }
}

impl eframe::App for EightQueensApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- Update Logic ---
        // --- Animation Update ---
        let dt = ctx.input(|i| i.stable_dt);
        self.particles.retain_mut(|p| {
            p.pos += p.vel * dt;
            p.vel.y += 800.0 * dt; // Gravity
            p.life -= dt * 1.5;
            p.life > 0.0
        });

        let delay_ms = if self.speed == 10 {
            0
        } else {
            (10 - self.speed) * 50
        };

        if self.auto_play && !self.solver.finished {
            if self.speed == 10 {
                let start = Instant::now();
                let mut found_any = false;
                while start.elapsed() < Duration::from_millis(16) && !self.solver.finished {
                    if self.solver.step() {
                        found_any = true;
                        if !self.finding_all {
                            self.paused = true;
                            self.auto_play = false;
                            break;
                        }
                    }
                }
                if found_any {
                    let center = ctx.screen_rect().center();
                    self.spawn_particles(center, self.theme.accent_color);
                }
                if self.solver.finished {
                    self.solver.restore_last_solution();
                }
                ctx.request_repaint();
            } else {
                if self.last_update.elapsed().as_millis() as u64 >= delay_ms {
                    if self.solver.step() {
                        let center = ctx.screen_rect().center();
                        self.spawn_particles(center, self.theme.accent_color);
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
                    let center = ctx.screen_rect().center();
                    self.spawn_particles(center, self.theme.accent_color);
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

        // --- Responsive Layout Detection ---
        let screen_rect = ctx.screen_rect();
        let is_mobile = screen_rect.width() < 700.0;

        if is_mobile {
            // Mobile: Minimal Top Header
            egui::TopBottomPanel::top("mobile_top")
                .frame(panel_frame.inner_margin(egui::Margin::symmetric(10.0, 5.0)))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("♛ N-Queens")
                                .strong()
                                .color(self.theme.accent_color),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "Sols: {}",
                                    self.solver.solutions.len()
                                ))
                                .strong(),
                            );
                        });
                    });
                });

            // Mobile: Compact Bottom Controls
            egui::TopBottomPanel::bottom("mobile_bottom")
                .frame(panel_frame.inner_margin(egui::Margin::symmetric(15.0, 10.0)))
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        // Row 1: Board Size & Speed
                        ui.horizontal(|ui| {
                            ui.label("Size:");
                            if ui.button("-").clicked() && self.n > 4 {
                                self.n -= 1;
                                self.n_input = self.n.to_string();
                                self.solver = SolverWrapper::new(self.n);
                                self.paused = true;
                                self.auto_play = false;
                            }
                            ui.label(
                                egui::RichText::new(self.n.to_string())
                                    .strong()
                                    .color(self.theme.accent_color),
                            );
                            if ui.button("+").clicked() && self.n < 30 {
                                self.n += 1;
                                self.n_input = self.n.to_string();
                                self.solver = SolverWrapper::new(self.n);
                                self.paused = true;
                                self.auto_play = false;
                            }

                            ui.add_space(20.0);
                            ui.label("Speed:");
                            ui.add(egui::Slider::new(&mut self.speed, 1..=10).show_value(true));
                        });

                        ui.add_space(8.0);

                        // Row 2: Options & Settings
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.only_unique, "Unique Only");
                            ui.checkbox(&mut self.show_threats, "Threats");

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("🎨 Theme").clicked() {
                                        let presets = Theme::presets();
                                        if let Some(idx) =
                                            presets.iter().position(|t| t.name == self.theme.name)
                                        {
                                            self.theme = presets[(idx + 1) % presets.len()].clone();
                                        }
                                    }

                                    if ui.button("� Export").clicked() {
                                        let display_solutions: Vec<String> = if self.only_unique {
                                            self.solver
                                                .solutions
                                                .iter()
                                                .filter(|s| !s.starts_with("(Sym)"))
                                                .cloned()
                                                .collect()
                                        } else {
                                            self.solver.solutions.clone()
                                        };
                                        #[cfg(target_arch = "wasm32")]
                                        web_csv_export(&display_solutions, self.n);
                                        #[cfg(not(target_arch = "wasm32"))]
                                        if let Some(path) = rfd::FileDialog::new()
                                            .add_filter("CSV", &["csv"])
                                            .set_file_name(&format!("nqueens_{}.csv", self.n))
                                            .save_file()
                                        {
                                            let mut wtr = csv::Writer::from_path(path).unwrap();
                                            let _ =
                                                wtr.write_record(&["Solution #", "Configuration"]);
                                            for (i, sol) in display_solutions.iter().enumerate() {
                                                let _ = wtr.write_record(&[
                                                    (i + 1).to_string(),
                                                    sol.clone(),
                                                ]);
                                            }
                                            let _ = wtr.flush();
                                        }
                                    }
                                },
                            );
                        });

                        ui.add_space(8.0);

                        // Row 3: Playback Controls
                        ui.horizontal_centered(|ui| {
                            let b_size = egui::vec2(ui.available_width() / 5.0 - 5.0, 45.0);
                            if ui.add_sized(b_size, egui::Button::new("▶")).clicked() {
                                if self.solver.finished {
                                    self.solver = SolverWrapper::new(self.n);
                                }
                                self.paused = false;
                                self.auto_play = false;
                                self.finding_all = false;
                            }
                            if ui.add_sized(b_size, egui::Button::new("|▶")).clicked() {
                                self.solver.step();
                                self.paused = true;
                            }
                            if ui.add_sized(b_size, egui::Button::new("⏩")).clicked() {
                                while !self.solver.finished {
                                    if self.solver.step() {
                                        break;
                                    }
                                }
                                self.paused = true;
                                self.solver.backtracking = true;
                            }
                            if ui.add_sized(b_size, egui::Button::new("⏭")).clicked() {
                                self.auto_play = true;
                                self.finding_all = true;
                                self.speed = 10;
                                self.paused = false;
                            }
                            if ui.add_sized(b_size, egui::Button::new("◼")).clicked() {
                                if !self.paused && !self.solver.finished {
                                    self.paused = true;
                                } else {
                                    self.solver = SolverWrapper::new(self.n);
                                    self.paused = true;
                                }
                            }
                        });
                    });
                });
        } else {
            // Desktop: Side Panel
            egui::SidePanel::right("controls")
                .frame(panel_frame)
                .min_width(320.0)
                .resizable(true)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new("♛ N-Queens")
                                    .size(24.0)
                                    .strong()
                                    .color(self.theme.text_color),
                            );
                        });
                        ui.add_space(20.0);
                        ui.label(
                            egui::RichText::new("Configuration")
                                .strong()
                                .color(self.theme.text_color),
                        );
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label("Board Size (4-30):");
                            let resp = ui.add(
                                egui::TextEdit::singleline(&mut self.n_input).desired_width(50.0),
                            );
                            if resp.changed() {
                                if let Ok(new_n) = self.n_input.parse::<usize>() {
                                    if new_n >= 4 && new_n <= 30 && new_n != self.n {
                                        self.n = new_n;
                                        self.solver = SolverWrapper::new(self.n);
                                        self.paused = true;
                                        self.auto_play = false;
                                    }
                                }
                            }
                            let should_update = resp.lost_focus()
                                || (resp.has_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                            if should_update {
                                self.n_input = self.n.to_string();
                            }
                        });
                        ui.add_space(15.0);
                        ui.label(
                            egui::RichText::new("Controls")
                                .strong()
                                .color(self.theme.text_color),
                        );
                        ui.separator();
                        ui.horizontal_wrapped(|ui| {
                            let btn_size = egui::vec2(50.0, 40.0);
                            if ui.add_sized(btn_size, egui::Button::new("▶")).clicked() {
                                if self.solver.finished {
                                    self.solver = SolverWrapper::new(self.n);
                                }
                                self.paused = false;
                                self.auto_play = false;
                                self.finding_all = false;
                            }
                            if ui.add_sized(btn_size, egui::Button::new("|▶")).clicked() {
                                self.solver.step();
                                self.paused = true;
                            }
                            if ui.add_sized(btn_size, egui::Button::new("⏩")).clicked() {
                                while !self.solver.finished {
                                    if self.solver.step() {
                                        break;
                                    }
                                }
                                self.paused = true;
                                self.solver.backtracking = true;
                            }
                            if ui.add_sized(btn_size, egui::Button::new("⏭")).clicked() {
                                self.auto_play = true;
                                self.finding_all = true;
                                self.speed = 10;
                                self.paused = false;
                            }
                            if ui.add_sized(btn_size, egui::Button::new("◼")).clicked() {
                                if !self.paused && !self.solver.finished {
                                    self.paused = true;
                                } else {
                                    self.solver = SolverWrapper::new(self.n);
                                    self.paused = true;
                                }
                            }
                        });

                        ui.add_space(10.0);
                        ui.label("Speed");
                        ui.add(egui::Slider::new(&mut self.speed, 1..=10).text("Speed"));

                        ui.add_space(10.0);
                        ui.checkbox(&mut self.show_threats, "Show Threatened Squares");
                        ui.checkbox(&mut self.only_unique, "Show Unique Solutions Only");

                        ui.add_space(10.0);
                        ui.label("Theme:");
                        egui::ComboBox::from_id_salt("theme_picker")
                            .selected_text(self.theme.name)
                            .show_ui(ui, |ui| {
                                for preset in Theme::presets() {
                                    ui.selectable_value(
                                        &mut self.theme,
                                        preset.clone(),
                                        preset.name,
                                    );
                                }
                            });

                        ui.add_space(20.0);
                        let display_solutions: Vec<String> = if self.only_unique {
                            self.solver
                                .solutions
                                .iter()
                                .filter(|s| !s.starts_with("(Sym)"))
                                .cloned()
                                .collect()
                        } else {
                            self.solver.solutions.clone()
                        };

                        ui.label(
                            egui::RichText::new(format!(
                                "Solutions Found: {}",
                                display_solutions.len()
                            ))
                            .strong()
                            .size(16.0),
                        );

                        ui.add_space(10.0);
                        if ui.button("Export to CSV").clicked() {
                            #[cfg(target_arch = "wasm32")]
                            web_csv_export(&display_solutions, self.n);
                            #[cfg(not(target_arch = "wasm32"))]
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("CSV", &["csv"])
                                .set_file_name(&format!("nqueens_{}.csv", self.n))
                                .save_file()
                            {
                                let mut wtr = csv::Writer::from_path(path).unwrap();
                                wtr.write_record(&["Solution #", "Configuration"]).unwrap();
                                for (i, sol) in display_solutions.iter().enumerate() {
                                    wtr.write_record(&[(i + 1).to_string(), sol.clone()])
                                        .unwrap();
                                }
                                wtr.flush().unwrap();
                            }
                        }

                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new("Solutions History")
                                .strong()
                                .color(self.theme.text_color),
                        );
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .max_height(200.0)
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                for (i, sol) in display_solutions.iter().enumerate() {
                                    ui.label(
                                        egui::RichText::new(format!("#{}: {}", i + 1, sol))
                                            .monospace()
                                            .size(12.0),
                                    );
                                }
                            });
                    });
                });
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(self.theme.background))
            .show(ctx, |ui| {
                let available_rect = ui.available_rect_before_wrap();
                let margin = if is_mobile { 10.0 } else { 60.0 };
                let size = (available_rect.height() - margin)
                    .min(available_rect.width() - margin)
                    .max(0.0);
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
                            let center = cell_rect.center();
                            let font_size = cell_size * 0.7;
                            let alpha = if row == self.solver.row && col == self.solver.col - 1 {
                                ctx.animate_bool(egui::Id::new((row, col)), true)
                            } else {
                                1.0
                            };

                            painter.text(
                                center,
                                egui::Align2::CENTER_CENTER,
                                "♛",
                                egui::FontId::proportional(font_size),
                                self.theme.queen_color.linear_multiply(alpha),
                            );
                        }
                    }
                }

                // Draw Particles
                for p in &self.particles {
                    painter.circle_filled(p.pos, p.size, p.color.linear_multiply(p.life));
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

#[cfg(target_arch = "wasm32")]
fn web_csv_export(solutions: &[String], n: usize) {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;
    let mut csv_content = String::from("Solution #,Configuration\n");
    for (i, sol) in solutions.iter().enumerate() {
        csv_content.push_str(&format!("{},\"{}\"\n", i + 1, sol));
    }
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let parts = js_sys::Array::of1(&JsValue::from_str(&csv_content));
    let blob = web_sys::Blob::new_with_str_sequence_and_options(
        &parts,
        web_sys::BlobPropertyBag::new().type_("text/csv"),
    )
    .unwrap();
    let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();
    let a = document
        .create_element("a")
        .unwrap()
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .unwrap();
    a.set_href(&url);
    a.set_download(&format!("nqueens_{}.csv", n));
    a.click();
    web_sys::Url::revoke_object_url(&url).unwrap();
}
