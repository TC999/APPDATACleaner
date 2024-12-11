use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path};

#[derive(Debug, Serialize, Deserialize)]
pub struct FolderDescriptions {
    pub name: String,  // 规则名称
    pub folders: HashMap<String, String>, // 文件夹描述
}

impl FolderDescriptions {
    // 加载 YAML 文件，支持传入路径
    pub fn load_from_yaml(file_path: &str) -> Result<Vec<Self>, String> {
        let path = Path::new(file_path);
        if !path.exists() {
            return Err("YAML 文件未找到".to_string());
        }

        let content = fs::read_to_string(path)
            .map_err(|e| format!("读取 YAML 文件失败: {}", e))?;

        let descriptions: Vec<FolderDescriptions> = serde_yaml::from_str(&content)
            .map_err(|e| format!("解析 YAML 文件失败: {}", e))?;

        Ok(descriptions)
    }

    // 获取指定文件夹的描述
    pub fn get_description(&self, folder_name: &str) -> Option<String> {
        self.folders.get(folder_name).cloned()
    }
}
