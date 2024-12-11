use eframe::egui;
use std::collections::HashMap;

pub struct SubscriptionManager {
    pub is_open: bool,
    pub subscriptions: HashMap<String, String>, // 显示已订阅规则，键为名称，值为描述或文件路径
    pub download_progress: Option<f32>,        // 下载进度
    pub download_status: Option<String>,       // 下载状态信息
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self {
            is_open: false,
            subscriptions: HashMap::new(),
            download_progress: None,
            download_status: None,
        }
    }
}

impl SubscriptionManager {
    pub fn show_window(&mut self, ctx: &egui::Context) {
        if self.is_open {
            egui::Window::new("订阅规则")
                .open(&mut self.is_open)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("下载规则").clicked() {
                            self.start_download_dialog(ui);
                        }
                        if ui.button("从文件导入").clicked() {
                            self.import_from_file();
                        }
                    });

                    ui.separator();
                    ui.label("已订阅规则：");
                    for (name, description) in &self.subscriptions {
                        ui.horizontal(|ui| {
                            ui.label(format!("{} - {}", name, description));
                            if ui.button("删除").clicked() {
                                self.subscriptions.remove(name);
                            }
                        });
                    }

                    if let Some(progress) = self.download_progress {
                        ui.add(egui::ProgressBar::new(progress).show_percentage());
                    }
                    if let Some(status) = &self.download_status {
                        ui.label(status);
                    }
                });
        }
    }

    fn start_download_dialog(&mut self, ui: &mut egui::Ui) {
        ui.label("请输入规则下载链接：");
        let mut url = String::new();
        ui.text_edit_singleline(&mut url);

        if ui.button("确定").clicked() {
            self.download_progress = Some(0.0);
            self.download_status = Some("开始下载...".to_string());
            // 模拟下载流程
            // TODO: 实现异步下载逻辑，更新进度并完成时移除弹窗
            self.download_progress = Some(1.0); // 下载完成
            self.download_status = Some("下载完成".to_string());
            self.subscriptions
                .insert("新规则".to_string(), "从网络下载".to_string());
        }
    }

    fn import_from_file(&mut self) {
        // TODO: 打开文件选择窗口，解析 YAML 并更新订阅规则
        // 示例添加一个规则
        self.subscriptions
            .insert("本地规则".to_string(), "从文件导入".to_string());
    }
}
