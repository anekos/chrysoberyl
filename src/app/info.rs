


#[derive(Default)]
pub struct AppInfo {
    pub active: bool,
    pub pages: usize,
    pub real_pages: usize,
    pub current_page: Option<usize>,
}


impl AppInfo {
    pub fn is_empty(&self) -> bool {
        self.current_page.is_none()
    }
}
