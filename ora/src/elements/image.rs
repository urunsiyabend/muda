use crate::element::{Element, LayoutContext, LayoutId, PaintContext, PrepaintContext};
use crate::rendering::texture::TextureId;
use crate::style::{Color, Style, Length, Background, Rect, Size};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default)]
pub enum ObjectFit {
    #[default]
    Contain,  // Scale to fit inside, preserve aspect ratio (letterbox)
    Cover,    // Scale to fill, preserve aspect ratio (crop)
    Fill,     // Stretch to fill (distort)
}

impl ObjectFit {
    /// Compute image bounds within container based on fit mode
    pub fn compute_bounds(&self, image_size: (u32, u32), container: &Rect) -> Rect {
        let (img_w, img_h) = (image_size.0 as f32, image_size.1 as f32);
        let cont_w = container.size.width;
        let cont_h = container.size.height;

        match self {
            ObjectFit::Fill => {
                // Stretch to fill container
                container.clone()
            }
            ObjectFit::Contain => {
                // Scale to fit inside, preserve aspect ratio
                let img_aspect = img_w / img_h;
                let cont_aspect = cont_w / cont_h;

                let (w, h) = if img_aspect > cont_aspect {
                    (cont_w, cont_w / img_aspect)
                } else {
                    (cont_h * img_aspect, cont_h)
                };

                let x = container.origin.x + (cont_w - w) / 2.0;
                let y = container.origin.y + (cont_h - h) / 2.0;
                Rect::new(x, y, w, h)
            }
            ObjectFit::Cover => {
                // Scale to fill, preserve aspect ratio (may crop)
                let img_aspect = img_w / img_h;
                let cont_aspect = cont_w / cont_h;

                let (w, h) = if img_aspect > cont_aspect {
                    (cont_h * img_aspect, cont_h)
                } else {
                    (cont_w, cont_w / img_aspect)
                };

                let x = container.origin.x + (cont_w - w) / 2.0;
                let y = container.origin.y + (cont_h - h) / 2.0;
                Rect::new(x, y, w, h)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum ImageLoadState {
    Pending,
    Loading,
    Loaded(TextureId, (u32, u32)), // id, (width, height)
    Error(String),
}

#[derive(Clone)]
pub enum ImageSourceKind {
    File(PathBuf),
    Bytes(Arc<[u8]>),
}

pub struct Image {
    source: ImageSourceKind,
    object_fit: ObjectFit,
    placeholder_color: Color,
    style: Style,
    load_state: ImageLoadState,
}

pub fn img(path: impl Into<PathBuf>) -> Image {
    Image::from_file(path.into())
}

pub fn img_from_bytes(bytes: impl Into<Arc<[u8]>>) -> Image {
    Image::from_bytes(bytes.into())
}

impl Image {
    pub fn from_file(path: PathBuf) -> Self {
        Self {
            source: ImageSourceKind::File(path),
            object_fit: ObjectFit::Contain,
            placeholder_color: Color::rgb(0.3, 0.3, 0.35), // Dark gray placeholder
            style: Style::default(),
            load_state: ImageLoadState::Pending,
        }
    }

    pub fn from_bytes(bytes: Arc<[u8]>) -> Self {
        Self {
            source: ImageSourceKind::Bytes(bytes),
            object_fit: ObjectFit::Contain,
            placeholder_color: Color::rgb(0.3, 0.3, 0.35),
            style: Style::default(),
            load_state: ImageLoadState::Pending,
        }
    }

    pub fn object_fit(mut self, fit: ObjectFit) -> Self {
        self.object_fit = fit;
        self
    }

    pub fn placeholder_color(mut self, color: Color) -> Self {
        self.placeholder_color = color;
        self
    }

    pub fn w(mut self, w: impl Into<Length>) -> Self {
        self.style.width = w.into();
        self
    }

    pub fn h(mut self, h: impl Into<Length>) -> Self {
        self.style.height = h.into();
        self
    }
}

pub struct ImageState {
    layout_id: LayoutId,
}

impl Element for Image {
    type RequestLayoutState = ImageState;

    fn request_layout(&mut self, cx: &mut LayoutContext) -> (LayoutId, Self::RequestLayoutState) {
        // If we have loaded dimensions, use them as intrinsic size
        let id = cx.request_layout(&self.style);

        if let ImageLoadState::Loaded(_, (w, h)) = &self.load_state {
            // Set intrinsic size from image dimensions (if no explicit size)
            if matches!(self.style.width, Length::Auto) && matches!(self.style.height, Length::Auto) {
                cx.set_intrinsic_size(id, Size::new(*w as f32, *h as f32));
            }
        }

        (id, ImageState { layout_id: id })
    }

    fn prepaint(&mut self, _state: &mut Self::RequestLayoutState, _cx: &mut PrepaintContext) {
        // Image loading would be triggered here in a full implementation
        // For MVP, images are loaded synchronously or pre-loaded
    }

    fn paint(&mut self, state: &mut Self::RequestLayoutState, cx: &mut PaintContext) {
        let bounds = cx.bounds(state.layout_id);

        match &self.load_state {
            ImageLoadState::Pending | ImageLoadState::Loading | ImageLoadState::Error(_) => {
                // Paint placeholder
                let mut style = Style::default();
                style.background = Background::Solid(self.placeholder_color);
                cx.paint_styled_rect(&style, &bounds);
            }
            ImageLoadState::Loaded(texture_id, image_size) => {
                // Compute image bounds based on object-fit
                let _image_bounds = self.object_fit.compute_bounds(*image_size, &bounds);

                // TODO: Paint texture at image_bounds
                // For MVP, this requires adding texture rendering to the GPU pipeline
                // Paint placeholder for now
                let mut style = Style::default();
                style.background = Background::Solid(self.placeholder_color);
                cx.paint_styled_rect(&style, &bounds);

                log::debug!(
                    "Image: Would paint texture {:?} at {:?} (object-fit: {:?})",
                    texture_id, bounds, self.object_fit
                );
            }
        }
    }
}
