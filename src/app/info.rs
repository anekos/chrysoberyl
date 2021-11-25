#[derive(Default)]
pub struct AppInfo {
    pub active: bool,
    pub pages: usize,
    pub real_pages: usize,
    pub current_page: Option<usize>,
}
