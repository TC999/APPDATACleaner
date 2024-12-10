use crate::about;
use crate::confirmation;
use crate::delete;
use crate::ignore;
use crate::logger; // 导入 logger 模块
use crate::move_module; // 导入移动模块
use crate::open;
use crate::scanner;
use crate::utils;
use eframe::egui::{self, CollapsingHeader, Grid, ScrollArea};
use std::collections::HashSet;
use std::sync::mpsc::{Receiver, Sender};

pub struct AppDataCleaner {
    is_scanning: bool,
    current_folder: Option<String>,
    folder_data: Vec<(String, u64)>,
    show_about_window: bool,                // 确保字段存在
    confirm_delete: Option<(String, bool)>, // 保存要确认删除的文件夹状态
    selected_appdata_folder: String,        // 新增字段
    tx: Option<Sender<(String, u64)>>,
    rx: Option<Receiver<(String, u64)>>,
    is_logging_enabled: bool,             // 控制日志是否启用
    previous_logging_state: bool,         // 记录上一次日志启用状态
    ignored_folders: HashSet<String>,     // 忽略文件夹集合
    move_module: move_module::MoveModule, // 移动模块实例
}

impl Default for AppDataCleaner {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            is_scanning: false,
            current_folder: None,
            folder_data: vec![],
            show_about_window: false,                       // 默认值
            confirm_delete: None,                           // 初始化为 None
            selected_appdata_folder: "Roaming".to_string(), // 默认值为 Roaming
            tx: Some(tx),
            rx: Some(rx),
            is_logging_enabled: false,     // 默认禁用日志
            previous_logging_state: false, // 初始时假定日志系统未启用
            ignored_folders: ignore::load_ignored_folders(),
            move_module: Default::default(),
        }
    }
}

impl AppDataCleaner {
    fn setup_custom_fonts(&self, ctx: &egui::Context) {
        use eframe::egui::{FontData, FontDefinitions};

        let mut fonts = FontDefinitions::default();

        fonts.font_data.insert(
            "custom_font".to_owned(),
            FontData::from_static(include_bytes!("../assets/SourceHanSansCN-Regular.otf")),
        );

        fonts.families.insert(
            egui::FontFamily::Proportional,
            vec!["custom_font".to_owned()],
        );
        fonts
            .families
            .insert(egui::FontFamily::Monospace, vec!["custom_font".to_owned()]);

        ctx.set_fonts(fonts);
    }
}

// 添加文件夹的显示逻辑

impl eframe::App for AppDataCleaner {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.setup_custom_fonts(ctx);

        // 删除确认弹窗逻辑
        if let Some((folder_name, _)) = &self.confirm_delete {
            let message = format!("确定要彻底删除文件夹 {} 吗？", folder_name);
            logger::log_info(&message);
            if let Some(confirm) = confirmation::show_confirmation(ctx, &message) {
                if confirm {
                    if let Some(base_path) = utils::get_appdata_dir(&self.selected_appdata_folder) {
                        let full_path = base_path.join(folder_name);
                        if let Err(err) = delete::delete_folder(&full_path) {
                            eprintln!("Error: {}", err);
                            logger::log_error(&format!("Error: {}", err));
                        }
                    } else {
                        eprintln!("无法获取 {} 文件夹路径", self.selected_appdata_folder);
                        logger::log_error(&format!(
                            "无法获取 {} 文件夹路径",
                            self.selected_appdata_folder
                        ));
                    }
                }
                self.confirm_delete = None; // 清除状态
            }
        }

        // 顶部菜单
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            if ui.button("关于").clicked() {
                self.show_about_window = true; // 打开关于窗口
                ui.close_menu();
            }

            ui.separator();
            ui.checkbox(&mut self.is_logging_enabled, "启用日志");

            ui.menu_button("切换文件夹", |ui| {
                for folder in ["Roaming", "Local", "LocalLow"] {
                    if ui.button(folder).clicked() {
                        self.selected_appdata_folder = folder.to_string();
                        self.folder_data.clear();
                        self.is_scanning = false;
                        ui.close_menu();
                    }
                }
            });
            ui.label(format!("当前目标: {}", self.selected_appdata_folder));
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("立即扫描").clicked() && !self.is_scanning {
                self.is_scanning = true;
                self.folder_data.clear();

                let tx = self.tx.clone().unwrap();
                let folder_type = self.selected_appdata_folder.clone();

                scanner::scan_appdata(tx, &folder_type);
            }

            if let Some(rx) = &self.rx {
                while let Ok((folder, size)) = rx.try_recv() {
                    self.folder_data.push((folder, size));
                }
            }

            if self.is_scanning {
                ui.label("扫描中...");
            } else {
                ui.label("扫描完成");
            }

            ScrollArea::vertical().show(ui, |ui| {
                Grid::new("folders_table").striped(true).show(ui, |ui| {
                    ui.label("文件夹");
                    ui.label("大小");
                    ui.label("操作");
                    ui.end_row();

                    for (folder, size) in &self.folder_data {
                        // 创建折叠面板（CollapsingHeader）用于展示子文件夹
                        let mut is_expanded = false; // 这个变量控制折叠/展开状态
                        let header_response = CollapsingHeader::new(folder)
                            .default_open(true)
                            .show(ui, |ui| {
                                ui.label(format!("大小: {}", utils::format_size(*size)));

                                // 显示子文件夹（此部分模拟为静态数据，实际可以动态加载）
                                let subfolders = vec!["Subfolder 1", "Subfolder 2", "Subfolder 3"];
                                for subfolder in subfolders {
                                    ui.horizontal(|ui| {
                                        ui.label("→"); // 符号表示子文件夹
                                        ui.label(subfolder);
                                    });
                                }
                            });

                        // 使用 header_response 来检查是否展开
                        is_expanded = header_response.header_response.hovered();

                        // 如果当前文件夹有子文件夹，显示一个展开/折叠按钮
                        if !self.ignored_folders.contains(folder) {
                            if ui.button("彻底删除").clicked() {
                                self.confirm_delete = Some((folder.clone(), false));
                            }
                            if ui.button("移动").clicked() {
                                self.move_module.show_window = true;
                                self.move_module.folder_name = folder.clone();
                            }
                            if ui.button("忽略").clicked() {
                                self.ignored_folders.insert(folder.clone());
                                ignore::save_ignored_folders(&self.ignored_folders);
                                logger::log_info(&format!("文件夹 '{}' 已被忽略", folder));
                            }
                        } else {
                            ui.add_enabled(false, |ui: &mut egui::Ui| {
                                let response1 = ui.button("彻底删除");
                                let response2 = ui.button("移动");
                                let response3 = ui.button("忽略");
                                response1 | response2 | response3 // 返回合并的 Response
                            });
                        }
                        if ui.button("打开").clicked() {
                            if let Some(base_path) =
                                utils::get_appdata_dir(&self.selected_appdata_folder)
                            {
                                let full_path = base_path.join(folder);
                                if let Err(err) = open::open_folder(&full_path) {
                                    logger::log_error(&format!("无法打开文件夹: {}", err));
                                }
                            }
                        }
                        ui.end_row();
                    }
                });
            });
        });

        // 关于窗口
        if self.show_about_window {
            about::show_about_window(ctx, &mut self.show_about_window);
        }

        // 显示移动窗口
        self.move_module.show_move_window(ctx);
    }
}
