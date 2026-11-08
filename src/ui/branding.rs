//! Decode the embedded window icon without filesystem access.

pub(crate) fn window_icon() -> Option<iced::window::Icon> {
    let bytes = include_bytes!("../../assets/window-icon.png");
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let mut pixels = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut pixels).ok()?;
    if info.color_type != png::ColorType::Rgba {
        return None;
    }
    pixels.truncate(info.buffer_size());
    iced::window::icon::from_rgba(pixels, info.width, info.height).ok()
}

#[cfg(test)]
mod tests;
