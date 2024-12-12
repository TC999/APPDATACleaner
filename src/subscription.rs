use eframe::egui;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub struct SubscriptionManager {
    pub is_open: bool,
    pub subscriptions: HashMap<String, String>, // 显示已订阅规则，键为名称，值为描述或文件路径
    pub download_progress: Option<f32>,         // 下载进度
    pub download_status: Option<String>,        // 下载状态信息
    pub download_url: String,                   // 当前输入的下载链接
    pub start_download_request: bool,           // 标志是否触发下载
    pub rules_directory: PathBuf,               // 规则文件存储目录
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        let rules_directory = PathBuf::from("./rules");

        // 确保目录存在
        if let Err(e) = fs::create_dir_all(&rules_directory) {
            eprintln!("无法创建规则目录: {}", e);
        }

        Self {
            is_open: false,
            subscriptions: HashMap::new(),
            download_progress: None,
            download_status: None,
            download_url: String::new(),
            start_download_request: false,
            rules_directory,
        }
    }
}

impl SubscriptionManager {
    pub fn set_rules_directory(&mut self, directory: &str) {
        let new_directory = PathBuf::from(directory);

        if let Err(e) = fs::create_dir_all(&new_directory) {
            eprintln!("无法创建规则目录: {}", e);
        } else {
            self.rules_directory = new_directory;
        }
    }

    pub fn show_window(&mut self, ctx: &egui::Context) {
        let mut is_open = self.is_open;
        if is_open {
            egui::Window::new("订阅规则")
                .open(&mut is_open)
                .show(ctx, |ui| {
                    let start_download = self.render_controls(ui);
                    self.render_subscriptions(ui);
                    self.render_download_status(ui);

                    if start_download {
                        self.handle_start_download();
                    }
                });
        }
        self.is_open = is_open;
    }

    fn render_controls(&mut self, ui: &mut egui::Ui) -> bool {
        let mut start_download = false;

        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.download_url).hint_text("请输入规则下载链接"),
            );

            if ui.button("下载规则").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter))
            {
                start_download = true;
            }

            if ui.button("从文件导入").clicked() {
                self.import_from_file();
            }
        });
        ui.separator();

        start_download
    }

    fn render_subscriptions(&mut self, ui: &mut egui::Ui) {
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

    fn render_download_status(&mut self, ui: &mut egui::Ui) {
        if let Some(progress) = self.download_progress {
            ui.add(egui::ProgressBar::new(progress).show_percentage());
        }
        if let Some(status) = &self.download_status {
            ui.label(status);
        }
    }

    fn handle_start_download(&mut self) {
        if self.download_url.is_empty() {
            self.download_status = Some("请输入有效的下载链接".to_string());
            return;
        }

        self.download_progress = Some(0.0);
        self.download_status = Some("开始下载...".to_string());

        // 模拟异步下载逻辑
        self.download_rule();
    }

    fn download_rule(&mut self) {
        let file_name = self
            .download_url
            .split('/')
            .last()
            .unwrap_or("default_rule.yaml");
        let file_path = self.rules_directory.join(file_name);

        // 模拟下载过程
        self.download_progress = Some(1.0); // 模拟下载完成

        if let Err(e) = fs::write(&file_path, "示例规则内容") {
            self.download_status = Some(format!("下载失败: {}", e));
        } else {
            self.download_status = Some("下载完成".to_string());
            self.subscriptions.insert(
                file_name.to_string(),
                format!("已保存到: {}", file_path.display()),
            );
        }
    }

    fn import_from_file(&mut self) {
        // TODO: 打开文件选择窗口，解析 YAML 并更新订阅规则
        self.subscriptions
            .insert("本地规则".to_string(), "从文件导入".to_string());
    }
}
