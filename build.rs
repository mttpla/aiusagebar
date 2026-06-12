fn main() {
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=11.0");

    generate_about_icon();

    vergen_git2::Emitter::default()
        .add_instructions(
            &vergen_git2::Git2Builder::default()
                .describe(true, true, None)
                .build()
                .unwrap(),
        )
        .unwrap()
        .emit()
        .unwrap();
}

fn generate_about_icon() {
    use ab_glyph::{Font, FontRef, Glyph, PxScale, ScaleFont, point};
    use image::{ImageBuffer, Rgba};

    const SIZE: u32 = 128;

    let version = std::env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION");
    let text = format!("[{}]", version);

    let font_bytes = std::fs::read("assets/fonts/CourierPrime-Bold.ttf")
        .expect("failed to read assets/fonts/CourierPrime-Bold.ttf");
    let font = FontRef::try_from_slice(&font_bytes).expect("failed to parse font");

    // Measure total advance at scale 1.0, then compute scale to hit 80% canvas width
    let scaled_1 = font.as_scaled(PxScale::from(1.0));
    let width_at_1: f32 = text
        .chars()
        .map(|c| scaled_1.h_advance(font.glyph_id(c)))
        .sum();
    let font_scale_value = (SIZE as f32 * 0.80) / width_at_1;
    let font_scale = PxScale::from(font_scale_value);
    let scaled = font.as_scaled(font_scale);

    // Horizontal centering: total advance at final scale
    let total_advance: f32 = text
        .chars()
        .map(|c| scaled.h_advance(font.glyph_id(c)))
        .sum();
    let start_x = (SIZE as f32 - total_advance) / 2.0;

    // Vertical centering: baseline so the full glyph block is centred
    let ascent = scaled.ascent();
    let descent = scaled.descent(); // negative
    let text_height = ascent - descent;
    let baseline_y = (SIZE as f32 - text_height) / 2.0 + ascent;

    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(SIZE, SIZE);
    let mut caret_x = start_x;

    for c in text.chars() {
        let glyph_id = font.glyph_id(c);
        let glyph = Glyph {
            id: glyph_id,
            scale: font_scale,
            position: point(caret_x, baseline_y),
        };
        caret_x += scaled.h_advance(glyph_id);
        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|x, y, coverage| {
                let px = bounds.min.x as i32 + x as i32;
                let py = bounds.min.y as i32 + y as i32;
                if px >= 0 && px < SIZE as i32 && py >= 0 && py < SIZE as i32 {
                    img.put_pixel(
                        px as u32,
                        py as u32,
                        Rgba([0, 0, 0, (coverage * 255.0) as u8]),
                    );
                }
            });
        }
    }

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR");
    let icon_path = std::path::Path::new(&out_dir).join("about-icon.png");
    img.save(&icon_path).expect("failed to write about-icon.png");

    println!("cargo:rerun-if-changed=assets/fonts/CourierPrime-Bold.ttf");
    println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");
}
