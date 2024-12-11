use eframe::egui;
use std::collections::HashMap;

pub struct SubscriptionManager {
    pub is_open: bool,
    pub subscriptions: HashMap<String, String>, // 显示已订阅规则，键为名称，值为描述或文件路径
    pub download_progress: Option<f32>,         // 下载进度
    pub download_status: Option<String>,        // 下载状态信息
    pub download_url: String,                   // 当前输入的下载链接
    pub start_download: bool,                   // 标志是否触发下载
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self {
            is_open: false,
            subscriptions: HashMap::new(),
            download_progress: None,
            download_status: None,
            download_url: String::new(),
            start_download: false,
        }
    }
}

impl SubscriptionManager {
    pub fn show_window(&mut self, ctx: &egui::Context) {
        if self.is_open {
            egui::Window::new("订阅规则")
                .open(&mut self.is_open)
                .show(ctx, |ui| {
                    self.show_controls(ui);
                    self.show_subscriptions(ui);
                    self.show_download_status(ui);
                });

            // 在 UI 渲染完成后处理状态更新
            if self.start_download {
                self.start_download();
                self.start_download = false;
            }
        }
    }

    fn show_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.download_url).hint_text("请输入规则下载链接"),
            );

            if ui.button("下载规则").clicked() {
                self.start_download = true;
            }

            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.start_download = true;
            }

            if ui.button("从文件导入").clicked() {
                self.import_from_file();
            }
        });
        ui.separator();
    }

    fn show_subscriptions(&mut self, ui: &mut egui::Ui) {
        ui.label("已订阅规则：");
        let subscriptions_to_remove: Vec<String> = self
            .subscriptions
            .iter()
            .filter_map(|(name, description)| {
                let mut remove = false;
                ui.horizontal(|ui| {
                    ui.label(format!("{} - {}", name, description));
                    if ui.button("删除").clicked() {
                        remove = true;
                    }
                });
                if remove {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        for name in subscriptions_to_remove {
            self.subscriptions.remove(&name);
        }
    }

    fn show_download_status(&mut self, ui: &mut egui::Ui) {
        if let Some(progress) = self.download_progress {
            ui.add(egui::ProgressBar::new(progress).show_percentage());
        }
        if let Some(status) = &self.download_status {
            ui.label(status);
        }
    }

    fn start_download(&mut self) {
        if self.download_url.is_empty() {
            self.download_status = Some("请输入有效的下载链接".to_string());
            return;
        }

        self.download_progress = Some(0.0);
        self.download_status = Some("开始下载...".to_string());

        // 模拟异步下载逻辑
        self.simulate_download();
    }

    fn simulate_download(&mut self) {
        // 这里是模拟的下载过程，你可以改为异步任务
        self.download_progress = Some(1.0); // 下载完成
        self.download_status = Some("下载完成".to_string());
        self.subscriptions
            .insert("新规则".to_string(), "从网络下载".to_string());
    }

    fn import_from_file(&mut self) {
        // TODO: 打开文件选择窗口，解析 YAML 并更新订阅规则
        self.subscriptions
            .insert("本地规则".to_string(), "从文件导入".to_string());
    }
}
