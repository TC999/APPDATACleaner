use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use std::sync::mpsc::Sender;  // 添加 Sender 导入
use std::error::Error;        // 添加标准错误特征
use crate::logger;

// 为 AIConfig 添加 Clone trait
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AIConfig {
    pub name: String,
    pub model: ModelConfig,
    pub retry: RetryConfig,
    pub Local: HashMap<String, String>,
    pub LocalLow: HashMap<String, String>,
    pub Roaming: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelConfig {
    pub url: String,
    pub api_key: String,
    pub model: String,
    pub prompt: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RetryConfig {
    pub attempts: u32,
    pub delay: u32,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            model: ModelConfig {
                url: "https://open.bigmodel.cn/api/paas/v4/chat/completions".to_string(),
                api_key: "your_api_key_here".to_string(),
                model: "glm-4-flash".to_string(),
                prompt: r#"    # 角色：Windows AppData分析专家

您是一个专业的Windows AppData文件夹分析专家。您需要分析用户提供的AppData文件夹信息并按照固定格式回答。

## 输入格式验证规则
当用户输入包含以下要素时视为有效：
1. 包含"AppData"关键词
2. 包含主目录[Local|LocalLow|Roaming]之一
3. 包含具体的应用程序文件夹名称

## 输出格式
```
- 软件名称：<应用程序名称>
- 数据类别：[配置|缓存|用户数据|日志]
- 应用用途：<简要描述（限50字）>
- 管理建议：[是|否]可安全删除
```

## 示例对话
用户输入：请分析Windows系统中AppData下Local文件夹中的Microsoft文件夹

系统输出：
- 软件名称：Microsoft Office
- 数据类别：配置
- 应用用途：存储Office应用程序的本地设置和临时文件
- 管理建议：是可安全删除

## 处理指令
1. 对任何符合输入格式的查询，直接使用输出格式回答
2. 保持输出格式的严格一致性
3. 不添加任何额外解释或评论
4. 确保应用用途描述在50字以内

## 注意
仅当输入完全不符合格式要求时，才返回："请按照正确的输入格式提供查询信息""#
                    .to_string(),
            },
            retry: RetryConfig {
                attempts: 3,
                delay: 20,
            },
            Local: HashMap::new(),
            LocalLow: HashMap::new(),
            Roaming: HashMap::new(),
        }
    }
}

impl AIConfig {
    pub fn new() -> Self {
        Self::default()
    }

    // 简化路径处理，直接使用固定路径
    pub fn get_config_path() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        Ok(std::path::PathBuf::from("folders_description.yaml"))
    }

    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: AIConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        // 序列化配置
        let content = serde_yaml::to_string(self)?;

        // 写入文件
        std::fs::write(path, content)?;
        
        logger::log_info(&format!("配置文件已保存到: {}", path));
        
        Ok(())
    }

    pub fn create_default_config(
        name: Option<String>,
        api_key: Option<String>,
        model: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut config = Self::default();

        // 使用提供的参数更新配置
        if let Some(name) = name {
            config.name = name;
        }
        if let Some(api_key) = api_key {
            config.model.api_key = api_key;
        }
        if let Some(model) = model {
            config.model.model = model;
        }

        // 获取正确的配置文件路径
        let config_path = Self::get_config_path()?;
        config.save_to_file(config_path.to_str().unwrap())?;

        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("配置名称不能为空".to_string());
        }
        if self.model.url.trim().is_empty() {
            return Err("API地址不能为空".to_string());
        }
        if self.model.api_key.trim().is_empty() {
            return Err("API密钥不能为空".to_string());
        }
        if self.model.model.trim().is_empty() {
            return Err("模型名称不能为空".to_string());
        }
        Ok(())
    }
}

// API 请求相关结构体
#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub messages: Vec<Message>,
    pub model: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: Message,
}

// AI 客户端结构体
#[derive(Debug)]
pub struct AIClient {
    config: AIConfig,
    client: reqwest::Client,
}

// 实现 AIClient
impl AIClient {
    pub fn new(config: AIConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    // 发送请求到 AI API 并处理重试
    pub async fn get_folder_description(
        &self,
        dir_1: &str,
        dir_2: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut attempts = 0;
        let max_attempts = self.config.retry.attempts;
        let delay = Duration::from_secs(self.config.retry.delay as u64);

        loop {
            attempts += 1;
            match self.try_get_description(dir_1, dir_2).await {
                Ok(description) => return Ok(description),
                Err(e) => {
                    if attempts >= max_attempts {
                        return Err(format!("达到最大重试次数 {}: {}", max_attempts, e).into());
                    }
                    crate::logger::log_error(&format!(
                        "API请求失败 (尝试 {}/{}): {}，将在 {}s 后重试",
                        attempts, max_attempts, e, delay.as_secs()
                    ));
                    tokio::time::sleep(delay).await;
                    continue;
                }
            }
        }
    }

    // 单次请求实现
    async fn try_get_description(
        &self,
        dir_1: &str,
        dir_2: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let request = ChatRequest {
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: self.config.model.prompt.clone(),
                },
                Message {
                    role: "user".to_string(),
                    content: format!(
                        "请简述Windows系统中AppData下的[{}]文件夹中的[{}]子文件夹的用途。",
                        dir_1, dir_2
                    ),
                },
            ],
            model: self.config.model.model.clone(),
        };

        let response = self.client
            .post(&self.config.model.url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.config.model.api_key))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!(
                "API请求失败: {} - {}", 
                response.status(),
                response.text().await?
            ).into());
        }

        let chat_response: ChatResponse = response.json().await?;
        if let Some(choice) = chat_response.choices.first() {
            Ok(choice.message.content.clone())
        } else {
            Err("API返回空响应".into())
        }
    }

    // 测试连接
    pub async fn test_connection(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let request = ChatRequest {
            messages: vec![Message {
                role: "user".to_string(),
                content: "测试连接".to_string(),
            }],
            model: self.config.model.model.clone(),
        };

        let response = self.client
            .post(&self.config.model.url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.config.model.api_key))
            .json(&request)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!("API连接测试失败: {}", response.status()).into())
        }
    }
}

// 添加新的AI处理功能结构体
#[derive(Debug)]
pub struct AIHandler {
    config: AIConfig,
    client: AIClient,
    tx: Option<Sender<(String, String, String)>>,
}

impl AIHandler {
    pub fn new(config: AIConfig, tx: Option<Sender<(String, String, String)>>) -> Self {
        Self {
            client: AIClient::new(config.clone()),
            config,
            tx,
        }
    }

    // 生成单个文件夹的描述
    pub async fn generate_single_description(
        &mut self,
        folder_name: String,
        selected_folder: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        logger::log_info(&format!("开始为 {} 生成描述", folder_name));

        match self.client.get_folder_description(&selected_folder, &folder_name).await {
            Ok(description) => {
                logger::log_info(&format!(
                    "成功生成描述 - {}/{}: {}", 
                    selected_folder, 
                    folder_name, 
                    description
                ));
                
                // 更新配置
                match selected_folder.as_str() {
                    "Local" => { self.config.Local.insert(folder_name.clone(), description.clone()); }
                    "LocalLow" => { self.config.LocalLow.insert(folder_name.clone(), description.clone()); }
                    "Roaming" => { self.config.Roaming.insert(folder_name.clone(), description.clone()); }
                    _ => {}
                };

                // 保存配置并通知
                if let Err(e) = self.save_config_and_notify(&selected_folder, &folder_name, &description) {
                    logger::log_error(&format!("保存配置失败: {}", e));
                }
                Ok(())
            }
            Err(e) => {
                logger::log_error(&format!(
                    "生成描述失败 {}/{}: {}", 
                    selected_folder,
                    folder_name, 
                    e
                ));
                Err(e)
            }
        }
    }

    // 批量生成所有文件夹的描述
    pub async fn generate_all_descriptions(
        &mut self,
        folder_data: Vec<(String, u64)>,
        selected_folder: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for (folder, _) in folder_data {
            if let Err(e) = self.generate_single_description(folder.clone(), selected_folder.clone()).await {
                logger::log_error(&format!("处理文件夹 {} 时发生错误: {}", folder, e));
                continue;
            }
        }
        Ok(())
    }

    // 保存配置并通知UI更新
    fn save_config_and_notify(
        &self,
        selected_folder: &str,
        folder_name: &str,
        description: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(config_path) = AIConfig::get_config_path() {
            match self.config.save_to_file(config_path.to_str().unwrap()) {
                Ok(_) => {
                    logger::log_info("配置文件保存成功");
                    // 发送更新消息到 UI
                    if let Some(tx) = &self.tx {
                        let _ = tx.send((
                            selected_folder.to_string(),
                            folder_name.to_string(),
                            description.to_string(),
                        ));
                    }
                    Ok(())
                }
                Err(e) => {
                    logger::log_error(&format!("保存配置失败: {}", e));
                    Err(e.into())
                }
            }
        } else {
            Err("无法获取配置文件路径".into())
        }
    }

    // 测试API连接
    pub async fn test_connection(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.client.test_connection().await
    }
}