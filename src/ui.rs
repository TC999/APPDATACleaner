use crate::about;
use crate::ai_config::{AIConfig, AIHandler};
use crate::confirmation;
use crate::ignore;
use crate::logger;
use crate::move_module;
use crate::open;
use crate::scanner;
use crate::utils;
use crate::yaml_loader::{load_folder_descriptions, FolderDescriptions};
use eframe::egui::{self, Grid, ScrollArea};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, Sender};

pub struct AppDataCleaner {
    // 基础字段
    is_scanning: bool,
    current_folder: Option<String>,
    folder_data: Vec<(String, u64)>,
    selected_appdata_folder: String,
    tx: Option<Sender<(String, u64)>>,
    rx: Option<Receiver<(String, u64)>>,
    total_size: u64,

    // 界面状态字段
    show_about_window: bool,
    show_ai_config_window: bool,     // AI配置窗口显示状态
    show_prompt_editor: bool,        // Prompt编辑器显示状态 
    confirm_delete: Option<(String, bool)>,
    status: Option<String>,          // 状态信息

    // 日志相关字段
    is_logging_enabled: bool,
    previous_logging_state: bool,

    // 排序相关字段 
    sort_criterion: Option<String>,  // 排序标准:"name"或"size"
    sort_order: Option<String>,      // 排序顺序:"asc"或"desc"

    // 文件夹描述相关
    folder_descriptions: Option<FolderDescriptions>,
    yaml_error_logged: bool,
    ignored_folders: HashSet<String>,

    // 移动模块
    move_module: move_module::MoveModule,

    // AI相关配置
    ai_config: AIConfig,
    ai_tx: Option<Sender<(String, String, String)>>,
    ai_rx: Option<Receiver<(String, String, String)>>,
    ai_handler: Arc<Mutex<AIHandler>>, // 使用 Arc<Mutex<>> 包装 AIHandler
}

impl Default for AppDataCleaner {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let (ai_tx, ai_rx) = std::sync::mpsc::channel();
        
        // 加载AI配置
        let ai_config = match AIConfig::load_from_file("folders_description.yaml") {
            Ok(config) => {
                logger::log_info("已成功加载AI配置文件");
                config
            }
            Err(_) => {
                logger::log_info("未找到配置文件，使用默认配置");
                AIConfig::default()
            }
        };

        // 创建 AIHandler 并包装在 Arc<Mutex<>> 中
        let ai_handler = Arc::new(Mutex::new(AIHandler::new(
            ai_config.clone(),
            Some(ai_tx.clone())
        )));

        Self {
            // 基础字段初始化
            is_scanning: false,
            current_folder: None,
            folder_data: vec![],
            selected_appdata_folder: "Roaming".to_string(),
            tx: Some(tx),
            rx: Some(rx),
            total_size: 0,

            // 界面状态初始化
            show_about_window: false,
            show_ai_config_window: false,
            show_prompt_editor: false,
            confirm_delete: None,
            status: Some("未扫描".to_string()),

            // 日志相关初始化
            is_logging_enabled: false,
            previous_logging_state: false,

            // 排序相关初始化
            sort_criterion: None,
            sort_order: None,

            // 文件夹描述相关初始化
            folder_descriptions: None,
            yaml_error_logged: false,
            ignored_folders: ignore::load_ignored_folders(),

            // 移动模块初始化
            move_module: Default::default(),

            // AI相关初始化
            ai_handler,
            ai_config,
            ai_tx: Some(ai_tx),
            ai_rx: Some(ai_rx),
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

impl eframe::App for AppDataCleaner {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.setup_custom_fonts(ctx);

        // 加载描述文件
        if self.folder_descriptions.is_none() {
            self.folder_descriptions =
                load_folder_descriptions("folders_description.yaml", &mut self.yaml_error_logged);
        }

        if self.is_logging_enabled != self.previous_logging_state {
            logger::init_logger(self.is_logging_enabled); // 初始化日志系统
            if self.is_logging_enabled {
                logger::log_info("日志系统已启用");
            } else {
                logger::log_info("日志系统已禁用");
            }
            self.previous_logging_state = self.is_logging_enabled; // 更新状态
        }

        // 删除确认弹窗逻辑
        confirmation::handle_delete_confirmation(
            ctx,
            &mut self.confirm_delete,
            &self.selected_appdata_folder,
            &mut self.status,
            &mut self.folder_data,
        ); // 传递 folder_data

        // 顶部菜单
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {  // 使用 horizontal 布局让按钮并排
                if ui.button("关于").clicked() {
                    self.show_about_window = true;
                    ui.close_menu();
                }
                if ui.button("AI配置").clicked() {
                    self.show_ai_config_window = true;
                    ui.close_menu();
                }
                if ui.button("一键生成所有描述").clicked() {
                    let folder_data = self.folder_data.clone();
                    let selected_folder = self.selected_appdata_folder.clone();
                    let handler = self.ai_handler.clone(); // 克隆Arc<Mutex<>>
                    
                    self.status = Some("正在生成描述...".to_string());
                    
                    std::thread::spawn(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        rt.block_on(async {
                            if let Ok(mut handler) = handler.lock() {
                                if let Err(e) = handler.generate_all_descriptions(folder_data, selected_folder).await {
                                    logger::log_error(&format!("批量生成描述失败: {}", e));
                                }
                            }
                        });
                    });
                }
            });

            ui.separator();
            ui.checkbox(&mut self.is_logging_enabled, "启用日志");

            ui.menu_button("切换文件夹", |ui| {
                for folder in ["Roaming", "Local", "LocalLow"] {
                    if ui.button(folder).clicked() {
                        self.selected_appdata_folder = folder.to_string();
                        self.folder_data.clear();
                        self.is_scanning = false;
                        self.status = Some("未扫描".to_string()); // 更新状态为 "未扫描"
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
                self.status = Some("扫描中...".to_string()); // 更新状态为 "扫描中..."

                let tx = self.tx.clone().unwrap();
                let folder_type = self.selected_appdata_folder.clone();

                scanner::scan_appdata(tx, &folder_type);
            }

            if let Some(rx) = &self.rx {
                while let Ok((folder, size)) = rx.try_recv() {
                    // 检查是否接收到扫描完成标志
                    if folder == "__SCAN_COMPLETE__" {
                        self.is_scanning = false;
                        self.status = Some("扫描完成".to_string()); // 更新状态为 "扫描完成"
                    } else {
                        self.folder_data.push((folder, size));
                    }
                }
            }

            // 显示状态
            if let Some(status) = &self.status {
                ui.label(status);
            }

            // 添加排序按钮
            ui.menu_button("排序", |ui| {
                if ui.button("名称正序").clicked() {
                    self.sort_criterion = Some("name".to_string());
                    self.sort_order = Some("asc".to_string());
                }
                if ui.button("大小正序").clicked() {
                    self.sort_criterion = Some("size".to_string());
                    self.sort_order = Some("asc".to_string());
                }
                if ui.button("名称倒序").clicked() {
                    self.sort_criterion = Some("name".to_string());
                    self.sort_order = Some("desc".to_string());
                }
                if ui.button("大小倒序").clicked() {
                    self.sort_criterion = Some("size".to_string());
                    self.sort_order = Some("desc".to_string());
                }
            });

            // 计算总大小
            self.total_size = self.folder_data.iter().map(|(_, size)| size).sum();

            // 显示总大小
            ui.label(format!("总大小: {}", utils::format_size(self.total_size)));

            ScrollArea::vertical().show(ui, |ui| {
                Grid::new("folders_table").striped(true).show(ui, |ui| {
                    ui.label("文件夹");
                    ui.label("大小");
                    ui.label("描述");
                    ui.label("操作");
                    ui.end_row();

                    if let Some(criterion) = &self.sort_criterion {
                        self.folder_data.sort_by(|a, b| {
                            if *criterion == "name" {
                                if self.sort_order == Some("asc".to_string()) {
                                    a.0.cmp(&b.0)
                                } else {
                                    b.0.cmp(&a.0)
                                }
                            } else {
                                if self.sort_order == Some("asc".to_string()) {
                                    a.1.cmp(&b.1)
                                } else {
                                    b.1.cmp(&a.1)
                                }
                            }
                        });
                    }

                    for (folder, size) in &self.folder_data {
                        if self.ignored_folders.contains(folder) {
                            ui.add_enabled(
                                false,
                                egui::Label::new(
                                    egui::RichText::new(folder).color(egui::Color32::GRAY),
                                ),
                            );
                        } else {
                            ui.label(folder);
                        }
                        ui.label(utils::format_size(*size));

                        // 读取描述信息并显示
                        let description = self.folder_descriptions.as_ref().and_then(|desc| {
                            desc.get_description(folder, &self.selected_appdata_folder)
                        });
                        if let Some(desc) = description {
                            ui.label(desc);
                        } else {
                            ui.label("无描述");
                        }

                        if !self.ignored_folders.contains(folder) {
                            if ui.button("彻底删除").clicked() {
                                self.confirm_delete = Some((folder.clone(), false));
                                self.status = None; // 每次点击"彻底删除"时清除状态
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
                        if ui.button("生成描述").clicked() {
                            let folder_name = folder.clone();
                            let selected_folder = self.selected_appdata_folder.clone();
                            let handler = self.ai_handler.clone(); // 克隆Arc<Mutex<>>
                            
                            self.status = Some(format!("正在为 {} 生成描述...", folder_name));
                            
                            std::thread::spawn(move || {
                                let rt = tokio::runtime::Runtime::new().unwrap();
                                rt.block_on(async {
                                    if let Ok(mut handler) = handler.lock() {
                                        if let Err(e) = handler.generate_single_description(folder_name.clone(), selected_folder).await {
                                            logger::log_error(&format!("生成描述失败: {}", e));
                                        }
                                    }
                                });
                            });
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

        // 新增：AI配置窗口
        if self.show_ai_config_window {
            egui::Window::new("AI配置")
                .resizable(true)
                .collapsible(true)
                .min_width(400.0)  // 添加最小宽度
                .min_height(500.0) // 添加最小高度
                .show(ctx, |ui| {
                    ui.heading("AI配置生成器");

                    // 基本配置
                    ui.group(|ui| {  // 将基本配置也放入组中
                        ui.heading("基本设置");
                        ui.horizontal(|ui| {
                            ui.label("配置名称：");
                            ui.add(egui::TextEdit::singleline(&mut self.ai_config.name)
                                .hint_text("输入配置名称")  // 添加提示文本
                                .desired_width(200.0));     // 设置输入框宽度
                        });
                    });
                    
                    // API配置组
                    ui.group(|ui| {
                        ui.heading("API设置");
                        ui.horizontal(|ui| {
                            ui.label("API地址：");
                            ui.add(egui::TextEdit::singleline(&mut self.ai_config.model.url)
                                .hint_text("输入 API 地址，如 https://api.openai.com/v1")
                                .desired_width(250.0));
                        });

                        ui.horizontal(|ui| {
                            ui.label("API密钥：");
                            ui.add(egui::TextEdit::singleline(&mut self.ai_config.model.api_key)
                                .password(true)
                                .hint_text("输入你的API密钥")
                                .desired_width(250.0));
                        });

                        ui.horizontal(|ui| {
                            ui.label("模型名称：");
                            ui.add(egui::TextEdit::singleline(&mut self.ai_config.model.model)
                                .hint_text("输入模型名称，如 gpt-3.5-turbo")
                                .desired_width(250.0));
                        });
                    });

                    // 重试配置组
                    ui.group(|ui| {
                        ui.heading("重试设置");
                        ui.horizontal(|ui| {
                            ui.label("重试次数：");
                            ui.add(egui::DragValue::new(&mut self.ai_config.retry.attempts)
                                .range(1..=10)  // 使用 range 替代 clamp_range
                                .speed(1)
                                .prefix("次数: "));
                        });

                        ui.horizontal(|ui| {
                            ui.label("重试延迟：");
                            ui.add(egui::DragValue::new(&mut self.ai_config.retry.delay)
                                .range(1..=60)  // 使用 range 替代 clamp_range
                                .speed(1)
                                .suffix(" 秒"));
                        });
                    });

                    // Prompt编辑器按钮
                    ui.group(|ui| {
                        ui.heading("Prompt设置");
                        if ui.button("编辑Prompt模板").clicked() {
                            self.show_prompt_editor = true;
                        }
                        // 显示当前prompt的预览
                        ui.label("当前模板预览：");
                        ui.add(egui::TextEdit::multiline(&mut self.ai_config.model.prompt.clone())
                            .desired_width(f32::INFINITY)
                            .desired_rows(3)
                            .interactive(false));  // 使用 interactive(false) 替代 read_only
                    });

                    ui.add_space(10.0);  // 添加一些间距

                    // 按钮组
                    ui.horizontal(|ui| {
                        if ui.button("保存配置").clicked() {
                            match self.ai_config.validate() {
                                Ok(_) => {
                                    match AIConfig::get_config_path() {
                                        Ok(config_path) => {
                                            match self.ai_config.save_to_file(config_path.to_str().unwrap()) {
                                                Ok(_) => {
                                                    logger::log_info(&format!(
                                                        "AI配置已保存到: {}",
                                                        config_path.display()
                                                    ));
                                                    self.status = Some("配置已保存".to_string());
                                                }
                                                Err(err) => {
                                                    logger::log_error(&format!(
                                                        "保存配置失败: {}, 路径: {}", 
                                                        err, 
                                                        config_path.display()
                                                    ));
                                                    self.status = Some("保存配置失败".to_string());
                                                }
                                            }
                                        }
                                        Err(err) => {
                                            logger::log_error(&format!("获取配置路径失败: {}", err));
                                            self.status = Some("保存配置失败".to_string());
                                        }
                                    }
                                }
                                Err(err) => {
                                    logger::log_error(&format!("配置验证失败: {}", err));
                                    self.status = Some(format!("错误: {}", err));
                                }
                            }
                        }

                        if ui.button("测试连接").clicked() {
                            let handler = self.ai_handler.clone(); // 克隆Arc<Mutex<>>
                            
                            tokio::runtime::Runtime::new()
                                .unwrap()
                                .block_on(async {
                                    if let Ok(handler) = handler.lock() {
                                        match handler.test_connection().await {
                                            Ok(_) => {
                                                logger::log_info("AI连接测试成功");
                                                self.status = Some("AI连接测试成功".to_string());
                                            }
                                            Err(err) => {
                                                logger::log_error(&format!("AI连接测试失败: {}", err));
                                                self.status = Some(format!("AI连接测试失败: {}", err));
                                            }
                                        }
                                    }
                                });
                        }

                        if ui.button("重置默认值").clicked() {
                            self.ai_config = AIConfig::default();
                        }

                        if ui.button("关闭").clicked() {
                            self.show_ai_config_window = false;
                        }
                    });
                });
        }

        // Prompt编辑器窗口也添加边界
        if self.show_prompt_editor {
            egui::Window::new("Prompt模板编辑器")
                .resizable(true)
                .min_width(600.0)
                .min_height(400.0)
                .show(ctx, |ui| {
                    ui.label("编辑Prompt模板：");
                    ui.add_space(5.0);
                    
                    let mut prompt = self.ai_config.model.prompt.clone();
                    ui.add(
                        egui::TextEdit::multiline(&mut prompt)
                            .desired_width(f32::INFINITY)
                            .desired_rows(20)
                            .font(egui::TextStyle::Monospace) // 使用等宽字体
                    );
                    self.ai_config.model.prompt = prompt;

                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("保存").clicked() {
                            self.show_prompt_editor = false;
                        }
                        if ui.button("重置默认值").clicked() {
                            self.ai_config.model.prompt = AIConfig::default().model.prompt;
                        }
                        if ui.button("取消").clicked() {
                            self.show_prompt_editor = false;
                            self.ai_config.model.prompt = AIConfig::default().model.prompt;
                        }
                    });
                });
        }

        // 在主循环中处理接收到的更新
        if let Some(rx) = &self.ai_rx {
            while let Ok((folder_type, folder_name, description)) = rx.try_recv() {
                // 更新本地配置
                match folder_type.as_str() {
                    "Local" => { self.ai_config.Local.insert(folder_name.clone(), description.clone()); }
                    "LocalLow" => { self.ai_config.LocalLow.insert(folder_name.clone(), description.clone()); }
                    "Roaming" => { self.ai_config.Roaming.insert(folder_name.clone(), description.clone()); }
                    _ => {}
                };
                
                // 重新加载描述文件
                if let Ok(config) = AIConfig::load_from_file("folders_description.yaml") {
                    self.ai_config = config;
                    self.folder_descriptions = load_folder_descriptions("folders_description.yaml", &mut self.yaml_error_logged);
                    
                    // 更新状态
                    self.status = Some(format!("已更新 {} 的描述", folder_name));
                    
                    // 强制重绘
                    ctx.request_repaint();
                }
            }
        }

        // 显示移动窗口
        self.move_module.show_move_window(ctx);
    }
}