use macroquad::prelude::*;

const WINDOW_WIDTH: f32 = 1200.0;
const WINDOW_HEIGHT: f32 = 800.0;

const X_AXIS_WIDTH: i32 = 20;
const Y_AXIS_HEIGHT: i32 = 20;
const BASE_HALF_EXTENT: f32 = 10.0;
const MIN_ZOOM: f32 = 0.05;
const MAX_ZOOM: f32 = 1000.0;
const ZOOM_SPEED: f32 = 1.1;

fn window_conf() -> Conf {
    Conf {
        window_title: "Plotter".to_string(),
        window_width: WINDOW_WIDTH as i32,
        window_height: WINDOW_HEIGHT as i32,
        sample_count: 8,
        platform: miniquad::conf::Platform {
            swap_interval: Some(1),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut view = PlotView::default();
    let mut previous_mouse = None;

    loop {
        let (mouse_x, mouse_y) = mouse_position();
        let mouse = vec2(mouse_x, mouse_y);

        if is_mouse_button_down(MouseButton::Left) {
            if let Some(previous) = previous_mouse {
                view.pan(mouse - previous);
            }
            previous_mouse = Some(mouse);
        } else {
            previous_mouse = None;
        }

        let (_, wheel_y) = mouse_wheel();
        if wheel_y != 0.0 {
            view.zoom_at(mouse, wheel_y);
        }

        clear_background(BLACK);

        draw_axis_grid(&view);
        plot_line(function, &view);
        draw_cursor_value(function, &view, mouse);

        next_frame().await;
    }
}

fn function(x: f32) -> f32 {
    x.atan()
}

struct PlotView {
    center: Vec2,
    zoom: f32,
}

impl Default for PlotView {
    fn default() -> Self {
        Self {
            center: Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

impl PlotView {
    fn half_extent(&self) -> Vec2 {
        let extent = BASE_HALF_EXTENT / self.zoom;
        vec2(extent, extent)
    }

    fn pan(&mut self, mouse_delta: Vec2) {
        let extent = self.half_extent();
        self.center.x -= mouse_delta.x * 2.0 * extent.x / screen_width();
        self.center.y += mouse_delta.y * 2.0 * extent.y / screen_height();
    }

    fn zoom_at(&mut self, mouse: Vec2, wheel_delta: f32) {
        let old_extent = self.half_extent();
        let cursor_offset = vec2(
            mouse.x / screen_width() - 0.5,
            0.5 - mouse.y / screen_height(),
        );
        let cursor_world = self.center + cursor_offset * 2.0 * old_extent;

        self.zoom = (self.zoom * ZOOM_SPEED.powf(wheel_delta)).clamp(MIN_ZOOM, MAX_ZOOM);

        let new_extent = self.half_extent();
        self.center = cursor_world - cursor_offset * 2.0 * new_extent;
    }
}

fn plot_line(func: impl Fn(f32) -> f32, view: &PlotView) {
    let extent = view.half_extent();
    let min_x = view.center.x - extent.x;
    let max_x = view.center.x + extent.x;
    let sample_count = screen_width().ceil().max(1.0) as usize;
    let mut previous: Option<Vec2> = None;

    for sample in 0..=sample_count {
        let x = min_x + (max_x - min_x) * sample as f32 / sample_count as f32;
        let y = func(x);

        if y.is_finite() {
            let point = world_to_screen(vec2(x, y), view);
            if let Some(previous) = previous {
                draw_line(previous.x, previous.y, point.x, point.y, 1.0, WHITE);
            }
            previous = Some(point);
        } else {
            previous = None;
        }
    }
}

fn world_to_screen(point: Vec2, view: &PlotView) -> Vec2 {
    let extent = view.half_extent();
    vec2(
        (point.x - view.center.x + extent.x) / (2.0 * extent.x) * screen_width(),
        (view.center.y + extent.y - point.y) / (2.0 * extent.y) * screen_height(),
    )
}

fn draw_cursor_value(func: impl Fn(f32) -> f32, view: &PlotView, mouse: Vec2) {
    if !(0.0..=screen_width()).contains(&mouse.x) || !(0.0..=screen_height()).contains(&mouse.y) {
        return;
    }

    let extent = view.half_extent();
    let x = view.center.x + (mouse.x / screen_width() - 0.5) * 2.0 * extent.x;
    let y = func(x);
    if !y.is_finite() {
        return;
    }

    let point = world_to_screen(vec2(x, y), view);
    if (0.0..=screen_width()).contains(&point.x) && (0.0..=screen_height()).contains(&point.y) {
        draw_circle(point.x, point.y, 5.0, YELLOW);
    }

    let x_step = 2.0 * extent.x / X_AXIS_WIDTH as f32;
    let y_step = 2.0 * extent.y / Y_AXIS_HEIGHT as f32;
    let coordinates = format!("({}, {})", format_tick(x, x_step), format_tick(y, y_step));
    draw_text(&coordinates, mouse.x + 10.0, mouse.y + 10.0, 20.0, YELLOW);
}

fn draw_axis_grid(view: &PlotView) {
    for column in 0..=X_AXIS_WIDTH {
        let x = column as f32 / X_AXIS_WIDTH as f32 * screen_width();
        draw_line(x, 0.0, x, screen_height(), 1.0, Color::from_hex(0x333333));
    }
    for row in 0..=Y_AXIS_HEIGHT {
        let y = row as f32 / Y_AXIS_HEIGHT as f32 * screen_height();
        draw_line(0.0, y, screen_width(), y, 1.0, Color::from_hex(0x333333));
    }

    let origin = world_to_screen(Vec2::ZERO, view);
    if (0.0..=screen_width()).contains(&origin.x) {
        draw_line(origin.x, 0.0, origin.x, screen_height(), 1.0, GRAY);
    }
    if (0.0..=screen_height()).contains(&origin.y) {
        draw_line(0.0, origin.y, screen_width(), origin.y, 1.0, GRAY);
    }
}

fn format_tick(value: f32, step: f32) -> String {
    let precision = ((-step.log10()).ceil() as i32 + 2).clamp(0, 6) as usize;
    let value = if value.abs() < step.abs() * 0.00001 {
        0.0
    } else {
        value
    };
    let mut label = format!("{value:.precision$}");
    if label.contains('.') {
        while label.ends_with('0') {
            label.pop();
        }
        if label.ends_with('.') {
            label.pop();
        }
    }
    label
}
