//! Renders the app icon PNGs into `assets/`. Run with `cargo run --example make_icon`.

use image::{Rgba, RgbaImage, imageops::FilterType};

fn main() {
    let size = 1024u32;
    let s = size as f32;
    let mut img = RgbaImage::new(size, size);
    let margin = s * 0.09;
    let radius = s * 0.2;
    let waves: [(f32, [f32; 3]); 2] = [(2.0, [90.0, 200.0, 190.0]), (2.5, [240.0, 140.0, 110.0])];

    for (x, y, px) in img.enumerate_pixels_mut() {
        let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
        // Rounded-square mask with a 1px antialiased edge.
        let qx = (fx - s / 2.0).abs() - (s / 2.0 - margin - radius);
        let qy = (fy - s / 2.0).abs() - (s / 2.0 - margin - radius);
        let outside = (qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0)) - radius;
        let mask = (0.5 - outside).clamp(0.0, 1.0);
        if mask == 0.0 {
            continue;
        }
        let g = fy / s;
        let mut c = [18.0 + 10.0 * g, 20.0 + 12.0 * g, 30.0 + 18.0 * g];

        let inner = s - 2.0 * margin;
        let u = (fx - margin) / inner;
        for (cycles, color) in waves {
            let amp = inner * 0.2;
            let phase = std::f32::consts::TAU * cycles * u;
            let wy = s / 2.0 + amp * phase.sin();
            let slope = amp * std::f32::consts::TAU * cycles / inner * phase.cos();
            let d = (fy - wy).abs() / (1.0 + slope * slope).sqrt();
            let a = (s * 0.018 - d).clamp(0.0, 1.0) * 0.9;
            for i in 0..3 {
                c[i] = c[i] * (1.0 - a) + color[i] * a;
            }
        }
        *px = Rgba([c[0] as u8, c[1] as u8, c[2] as u8, (mask * 255.0) as u8]);
    }

    std::fs::create_dir_all("assets").unwrap();
    for out in [1024u32, 512, 256, 128, 64, 32] {
        let resized = if out == size {
            img.clone()
        } else {
            image::imageops::resize(&img, out, out, FilterType::Lanczos3)
        };
        resized.save(format!("assets/icon-{out}.png")).unwrap();
    }
    println!("wrote assets/icon-*.png");
}
