use dioxus::prelude::*;
use latex_ide_ui::{Button, ButtonVariant};
use pdfium_render::prelude::*;
use std::path::{Path, PathBuf};
use anyhow::Result;

pub mod viewer;
pub mod controls;
pub mod renderer;

pub use viewer::PDFViewer;
pub use controls::PDFControls;
pub use renderer::PDFRenderer;

#[derive(Clone, Debug, PartialEq)]
pub struct PDFDocument {
    pub path: PathBuf,
    pub pages: Vec<PDFPage>,
    pub current_page: usize,
    pub zoom: f32,
    pub total_pages: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PDFPage {
    pub page_number: usize,
    pub width: f32,
    pub height: f32,
    pub rendered_data: Option<Vec<u8>>, // PNG data
}

impl PDFDocument {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            pages: Vec::new(),
            current_page: 0,
            zoom: 1.0,
            total_pages: 0,
        }
    }
    
    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * 1.2).min(5.0);
    }
    
    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / 1.2).max(0.1);
    }
    
    pub fn fit_width(&mut self, container_width: f32) {
        if let Some(page) = self.pages.get(self.current_page) {
            self.zoom = container_width / page.width;
        }
    }
    
    pub fn fit_height(&mut self, container_height: f32) {
        if let Some(page) = self.pages.get(self.current_page) {
            self.zoom = container_height / page.height;
        }
    }
    
    pub fn next_page(&mut self) {
        if self.current_page + 1 < self.total_pages {
            self.current_page += 1;
        }
    }
    
    pub fn prev_page(&mut self) {
        if self.current_page > 0 {
            self.current_page -= 1;
        }
    }
    
    pub fn go_to_page(&mut self, page: usize) {
        if page < self.total_pages {
            self.current_page = page;
        }
    }
}