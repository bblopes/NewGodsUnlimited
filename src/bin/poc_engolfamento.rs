use macroquad::prelude::*;

const PANEL_WIDTH: f32 = 336.0;
const PREDATOR_RADIUS: f32 = 30.0;
const PREY_RADIUS: f32 = 22.0;
const REQUIRED_RADIUS_RATIO: f32 = 1.35;
const OSMOSIS_MARGIN: f32 = PREY_RADIUS * 0.15;
const APPROACH_SPEED: f32 = 56.0;
const SUCTION_SPEED: f32 = 125.0;
const ESCAPE_SPEED: f32 = 30.0;

const BACKGROUND: Color = Color::new(0.035, 0.043, 0.059, 1.0);
const SURFACE: Color = Color::new(0.075, 0.09, 0.12, 1.0);
const SURFACE_HOVER: Color = Color::new(0.12, 0.145, 0.19, 1.0);
const BORDER: Color = Color::new(0.20, 0.235, 0.30, 1.0);
const TEXT: Color = Color::new(0.91, 0.93, 0.96, 1.0);
const MUTED: Color = Color::new(0.42, 0.48, 0.56, 1.0);
const ACCENT: Color = Color::new(0.20, 0.66, 0.93, 1.0);
const PREDATOR: Color = Color::new(0.26, 0.71, 0.96, 1.0);
const PREY: Color = Color::new(0.96, 0.62, 0.26, 1.0);
const GREEN: Color = Color::new(0.12, 0.70, 0.33, 1.0);
const RED: Color = Color::new(0.88, 0.18, 0.20, 1.0);

#[derive(Clone, Copy)]
struct Visibility {
    physical_body: bool,
    contact_limit: bool,
    suction_limit: bool,
    containment_limit: bool,
    nuclei_distance: bool,
    force_vector: bool,
}

impl Default for Visibility {
    fn default() -> Self {
        Self {
            physical_body: true,
            contact_limit: true,
            suction_limit: true,
            containment_limit: true,
            nuclei_distance: true,
            force_vector: true,
        }
    }
}

#[derive(Clone, Copy)]
struct POCState {
    predator: Vec2,
    prey: Vec2,
    prey_alpha: f32,
    running: bool,
    suction_active: bool,
    engulfed: bool,
    visibility: Visibility,
}

impl POCState {
    fn new(arena: Rect) -> Self {
        let predator = vec2(arena.x + arena.w * 0.43, arena.y + arena.h * 0.53);
        Self {
            predator,
            prey: predator + vec2(165.0, 0.0),
            prey_alpha: 1.0,
            running: false,
            suction_active: false,
            engulfed: false,
            visibility: Visibility::default(),
        }
    }

    fn reset(&mut self, arena: Rect) {
        let visibility = self.visibility;
        *self = Self::new(arena);
        self.visibility = visibility;
    }

    fn tick(&mut self, delta: f32) {
        if !self.running {
            return;
        }
        if self.engulfed {
            self.prey_alpha = (self.prey_alpha - delta * 1.2).max(0.0);
            self.prey = self.prey.lerp(self.predator, (delta * 7.0).min(1.0));
            return;
        }
        let toward_predator = (self.predator - self.prey).normalize_or_zero();
        let suction_distance = PREDATOR_RADIUS + PREY_RADIUS + OSMOSIS_MARGIN;
        if self.predator.distance(self.prey) > suction_distance {
            self.prey += toward_predator * APPROACH_SPEED * delta;
            return;
        }
        self.suction_active = true;
        self.prey += toward_predator * (SUCTION_SPEED - ESCAPE_SPEED) * delta;
        if self.predator.distance(self.prey) <= PREDATOR_RADIUS - PREY_RADIUS {
            self.engulfed = true;
        }
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "POC — Engolfamento por Osmose".to_owned(),
        window_width: 1440,
        window_height: 900,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut state = POCState::new(arena_rect(panel_x()));
    loop {
        let panel_x = panel_x();
        let arena = arena_rect(panel_x);
        state.tick(get_frame_time());
        clear_background(BACKGROUND);
        draw_top_bar(panel_x);
        draw_arena(arena, state);
        draw_sidebar(panel_x, state, &mut state, arena);
        next_frame().await;
    }
}

fn panel_x() -> f32 {
    (screen_width() - PANEL_WIDTH).max(820.0)
}

fn arena_rect(panel_x: f32) -> Rect {
    let available_width = panel_x - 64.0;
    let available_height = screen_height() - 132.0;
    let width = available_width.min(available_height * 16.0 / 9.0);
    let height = width * 9.0 / 16.0;
    Rect::new(
        32.0,
        92.0 + (available_height - height).max(0.0) * 0.5,
        width,
        height,
    )
}

fn draw_top_bar(panel_x: f32) {
    draw_text("POC de engolfamento", 32.0, 40.0, 28.0, TEXT);
    draw_text(
        "Troubleshooting de contato, osmose e contenção geométrica",
        32.0,
        66.0,
        16.0,
        MUTED,
    );
    draw_line(
        panel_x - 20.0,
        20.0,
        panel_x - 20.0,
        screen_height() - 20.0,
        1.0,
        BORDER,
    );
}

fn draw_arena(arena: Rect, state: POCState) {
    draw_rectangle(
        arena.x,
        arena.y,
        arena.w,
        arena.h,
        Color::new(0.98, 0.985, 0.99, 1.0),
    );
    draw_rectangle_lines(arena.x, arena.y, arena.w, arena.h, 2.0, BORDER);
    draw_text(
        "Câmara de interação · unidades físicas em escala",
        arena.x + 18.0,
        arena.y + 30.0,
        17.0,
        MUTED,
    );

    let contact_distance = PREDATOR_RADIUS + PREY_RADIUS;
    let suction_distance = contact_distance + OSMOSIS_MARGIN;
    let containment_distance = PREDATOR_RADIUS - PREY_RADIUS;
    if state.visibility.suction_limit {
        draw_circle_lines(
            state.predator.x,
            state.predator.y,
            suction_distance,
            1.5,
            GREEN,
        );
        draw_text(
            "início da osmose",
            state.predator.x - suction_distance,
            state.predator.y - suction_distance - 7.0,
            14.0,
            GREEN,
        );
    }
    if state.visibility.contact_limit {
        draw_circle_lines(
            state.predator.x,
            state.predator.y,
            contact_distance,
            1.5,
            PREY,
        );
        draw_text(
            "contato físico",
            state.predator.x - contact_distance,
            state.predator.y + contact_distance + 17.0,
            14.0,
            PREY,
        );
    }
    if state.visibility.containment_limit {
        draw_circle_lines(
            state.predator.x,
            state.predator.y,
            containment_distance,
            1.5,
            ACCENT,
        );
        draw_text(
            "núcleo contido",
            state.predator.x + 12.0,
            state.predator.y - 13.0,
            13.0,
            ACCENT,
        );
    }
    if state.visibility.nuclei_distance {
        draw_distance_line(state.predator, state.prey);
    }
    if state.visibility.force_vector && state.suction_active && !state.engulfed {
        draw_force_arrow(state.prey, state.predator, GREEN, "osmose");
    }
    draw_cell(
        state.predator,
        PREDATOR_RADIUS,
        PREDATOR,
        1.0,
        "A · Osmose",
        state.visibility.physical_body,
    );
    if state.prey_alpha > 0.0 {
        draw_cell(
            state.prey,
            PREY_RADIUS,
            PREY,
            state.prey_alpha,
            "B · Fuga",
            state.visibility.physical_body,
        );
    }
    if state.engulfed {
        draw_text(
            "Engolfamento: núcleo de B contido em A",
            arena.x + 18.0,
            arena.y + arena.h - 20.0,
            18.0,
            GREEN,
        );
    }
}

fn draw_cell(position: Vec2, radius: f32, color: Color, alpha: f32, label: &str, show_body: bool) {
    let fill = Color::new(color.r, color.g, color.b, 0.88 * alpha);
    draw_circle(position.x, position.y, radius, fill);
    if show_body {
        draw_circle_lines(
            position.x,
            position.y,
            radius,
            2.0,
            Color::new(0.04, 0.14, 0.24, alpha),
        );
    }
    draw_circle(
        position.x - radius * 0.20,
        position.y - radius * 0.20,
        radius * 0.20,
        Color::new(0.95, 0.98, 1.0, 0.80 * alpha),
    );
    draw_text(
        label,
        position.x - radius,
        position.y + radius + 22.0,
        15.0,
        Color::new(0.05, 0.12, 0.20, alpha),
    );
}

fn draw_distance_line(from: Vec2, to: Vec2) {
    let distance = from.distance(to);
    draw_line(
        from.x,
        from.y,
        to.x,
        to.y,
        1.5,
        Color::new(0.18, 0.23, 0.32, 0.80),
    );
    let middle = (from + to) * 0.5;
    draw_rectangle(
        middle.x - 37.0,
        middle.y - 27.0,
        74.0,
        20.0,
        Color::new(1.0, 1.0, 1.0, 0.92),
    );
    draw_text(
        &format!("{distance:.1} u"),
        middle.x - 27.0,
        middle.y - 12.0,
        14.0,
        Color::new(0.10, 0.15, 0.22, 1.0),
    );
}

fn draw_force_arrow(from: Vec2, to: Vec2, color: Color, label: &str) {
    let direction = (to - from).normalize_or_zero();
    let end = to - direction * (PREDATOR_RADIUS + 12.0);
    let start = from + direction * (PREY_RADIUS + 10.0);
    draw_line(start.x, start.y, end.x, end.y, 3.0, color);
    draw_triangle(
        end,
        end - direction * 11.0 + direction.rotate(vec2(0.0, 1.0)) * 6.0,
        end - direction * 11.0 - direction.rotate(vec2(0.0, 1.0)) * 6.0,
        color,
    );
    let middle = (start + end) * 0.5;
    draw_text(label, middle.x - 24.0, middle.y - 10.0, 15.0, color);
}

fn draw_sidebar(panel_x: f32, state: POCState, state_mut: &mut POCState, arena: Rect) {
    let left = panel_x;
    let width = screen_width() - left - 16.0;
    draw_rectangle(
        panel_x - 2.0,
        0.0,
        PANEL_WIDTH + 2.0,
        screen_height(),
        SURFACE,
    );
    draw_text("Simulação de", left, 40.0, 22.0, TEXT);
    draw_text("engolfamento", left, 66.0, 22.0, TEXT);
    draw_text("POC isolada · sem evolução", left, 90.0, 14.0, MUTED);

    if button(
        Rect::new(left, 110.0, width * 0.48 - 4.0, 34.0),
        if state.running { "Pausar" } else { "Play" },
        state.running,
    ) {
        state_mut.running = !state_mut.running;
    }
    if button(
        Rect::new(left + width * 0.52 + 4.0, 110.0, width * 0.48 - 4.0, 34.0),
        "Reiniciar",
        false,
    ) {
        state_mut.reset(arena);
    }

    section(left, 176.0, "Troubleshooting visual");
    toggle_row(
        Rect::new(left, 190.0, width, 30.0),
        "Corpo físico",
        &mut state_mut.visibility.physical_body,
    );
    toggle_row(
        Rect::new(left, 224.0, width, 30.0),
        &format!("Contato · ≤ {:.1}u", PREDATOR_RADIUS + PREY_RADIUS),
        &mut state_mut.visibility.contact_limit,
    );
    toggle_row(
        Rect::new(left, 258.0, width, 30.0),
        &format!(
            "Início osmose · ≤ {:.1}u",
            PREDATOR_RADIUS + PREY_RADIUS + OSMOSIS_MARGIN
        ),
        &mut state_mut.visibility.suction_limit,
    );
    toggle_row(
        Rect::new(left, 292.0, width, 30.0),
        &format!("Engolfar · núcleo ≤ {:.1}u", PREDATOR_RADIUS - PREY_RADIUS),
        &mut state_mut.visibility.containment_limit,
    );
    toggle_row(
        Rect::new(left, 326.0, width, 30.0),
        "Linha entre núcleos",
        &mut state_mut.visibility.nuclei_distance,
    );
    toggle_row(
        Rect::new(left, 360.0, width, 30.0),
        "Vetor de força",
        &mut state_mut.visibility.force_vector,
    );

    section(left, 431.0, "Estado atual");
    let distance = state.predator.distance(state.prey);
    value_row(
        left,
        445.0,
        width,
        "Distância entre núcleos",
        &format!("{distance:.1} u"),
    );
    value_row(
        left,
        483.0,
        width,
        "Status",
        if state.engulfed {
            "Engolfada"
        } else if state.suction_active {
            "Osmose ativa"
        } else {
            "Aproximação"
        },
    );
    value_row(
        left,
        521.0,
        width,
        "Razão de raios",
        &format!(
            "{:.2}× / mínimo {:.2}×",
            PREDATOR_RADIUS / PREY_RADIUS,
            REQUIRED_RADIUS_RATIO
        ),
    );
    value_row(
        left,
        559.0,
        width,
        "Força líquida",
        &format!("{:.0} u/s", SUCTION_SPEED - ESCAPE_SPEED),
    );

    section(left, 632.0, "Rede de A · maior");
    draw_network(
        Rect::new(left, 646.0, width, 105.0),
        true,
        state.suction_active,
    );
    section(left, 780.0, "Rede de B · menor");
    draw_network(
        Rect::new(left, 794.0, width, 105.0),
        false,
        state.suction_active && !state.engulfed,
    );
}

fn toggle_row(rect: Rect, label: &str, value: &mut bool) {
    let (mouse_x, mouse_y) = mouse_position();
    let hovered = rect.contains(vec2(mouse_x, mouse_y));
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if hovered { SURFACE_HOVER } else { BACKGROUND },
    );
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, BORDER);
    draw_rectangle(
        rect.x + 9.0,
        rect.y + 7.0,
        16.0,
        16.0,
        if *value { ACCENT } else { BACKGROUND },
    );
    draw_rectangle_lines(
        rect.x + 9.0,
        rect.y + 7.0,
        16.0,
        16.0,
        1.0,
        if *value { ACCENT } else { MUTED },
    );
    if *value {
        draw_line(
            rect.x + 13.0,
            rect.y + 15.0,
            rect.x + 17.0,
            rect.y + 19.0,
            2.0,
            BACKGROUND,
        );
        draw_line(
            rect.x + 17.0,
            rect.y + 19.0,
            rect.x + 23.0,
            rect.y + 11.0,
            2.0,
            BACKGROUND,
        );
    }
    draw_text(label, rect.x + 34.0, rect.y + 20.0, 13.0, TEXT);
    if hovered && is_mouse_button_pressed(MouseButton::Left) {
        *value = !*value;
    }
}

fn draw_network(rect: Rect, attacker: bool, active: bool) {
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, BACKGROUND);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, BORDER);
    let sensor = vec2(rect.x + 36.0, rect.y + rect.h * 0.58);
    let internal = vec2(rect.x + rect.w * 0.50, rect.y + rect.h * 0.58);
    let motor = vec2(rect.x + rect.w - 36.0, rect.y + rect.h * 0.58);
    let color = if attacker { GREEN } else { RED };
    draw_link(sensor, internal, color, active);
    draw_link(internal, motor, color, active);
    draw_node(sensor, if attacker { "Ct" } else { "FçX" }, ACCENT);
    draw_node(internal, "I", Color::new(0.35, 0.48, 0.68, 1.0));
    draw_node(motor, if attacker { "Os" } else { "Fuga" }, color);
    draw_text("sensor", sensor.x - 18.0, rect.y + 17.0, 11.0, MUTED);
    draw_text("interno", internal.x - 19.0, rect.y + 17.0, 11.0, MUTED);
    draw_text("motor", motor.x - 17.0, rect.y + 17.0, 11.0, MUTED);
}

fn draw_link(from: Vec2, to: Vec2, color: Color, active: bool) {
    let direction = (to - from).normalize_or_zero();
    let end = to - direction * 13.0;
    let start = from + direction * 13.0;
    let line = if active {
        color
    } else {
        Color::new(0.43, 0.55, 0.72, 0.52)
    };
    draw_line(start.x, start.y, end.x, end.y, 2.5, line);
    draw_triangle(
        end,
        end - direction * 7.0 + direction.rotate(vec2(0.0, 1.0)) * 4.0,
        end - direction * 7.0 - direction.rotate(vec2(0.0, 1.0)) * 4.0,
        line,
    );
}

fn draw_node(position: Vec2, label: &str, color: Color) {
    draw_circle(position.x, position.y, 14.0, color);
    draw_circle_lines(position.x, position.y, 14.0, 1.0, BACKGROUND);
    let measure = measure_text(label, None, 11, 1.0);
    draw_text(
        label,
        position.x - measure.width * 0.5,
        position.y + 4.0,
        11.0,
        BACKGROUND,
    );
}

fn section(x: f32, y: f32, label: &str) {
    draw_text(label, x, y, 16.0, TEXT);
    draw_line(x, y + 8.0, x + PANEL_WIDTH - 18.0, y + 8.0, 1.0, BORDER);
}

fn value_row(x: f32, y: f32, width: f32, label: &str, value: &str) {
    draw_rectangle(x, y, width, 30.0, BACKGROUND);
    draw_text(label, x + 8.0, y + 20.0, 12.0, MUTED);
    let measure = measure_text(value, None, 12, 1.0);
    draw_text(value, x + width - measure.width - 8.0, y + 20.0, 12.0, TEXT);
}

fn button(rect: Rect, label: &str, selected: bool) -> bool {
    let (mouse_x, mouse_y) = mouse_position();
    let hovered = rect.contains(vec2(mouse_x, mouse_y));
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if selected {
            ACCENT
        } else if hovered {
            SURFACE_HOVER
        } else {
            BACKGROUND
        },
    );
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        if selected { ACCENT } else { BORDER },
    );
    let measure = measure_text(label, None, 14, 1.0);
    draw_text(
        label,
        rect.x + (rect.w - measure.width) * 0.5,
        rect.y + (rect.h + measure.height) * 0.5 - 2.0,
        14.0,
        if selected { BACKGROUND } else { TEXT },
    );
    hovered && is_mouse_button_pressed(MouseButton::Left)
}
