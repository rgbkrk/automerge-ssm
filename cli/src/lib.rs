#![allow(non_snake_case)]

use autosurgeon::{Hydrate, Reconcile};

#[derive(Debug, Clone, Reconcile, Hydrate)]
pub struct TodoItem {
    pub id: autosurgeon::Text,
    pub text: autosurgeon::Text,
    pub completed: bool,
}

pub fn hydrate_optional_timestamp<D: autosurgeon::ReadDoc>(
    doc: &D,
    obj: &automerge::ObjId,
    prop: autosurgeon::Prop,
) -> Result<Option<i64>, autosurgeon::HydrateError> {
    use automerge::Value;
    match doc.get(obj, &prop)? {
        Some((Value::Scalar(s), _)) => {
            match &*s {
                automerge::ScalarValue::Int(i) => Ok(Some(*i)),
                automerge::ScalarValue::Uint(u) => Ok(Some(*u as i64)),
                automerge::ScalarValue::Timestamp(t) => Ok(Some(*t)),
                _ => Ok(None),
            }
        }
        _ => Ok(None),
    }
}

pub fn hydrate_string_vec<D: autosurgeon::ReadDoc>(
    doc: &D,
    obj: &automerge::ObjId,
    prop: autosurgeon::Prop,
) -> Result<Vec<String>, autosurgeon::HydrateError> {
    use automerge::{ObjType, Value};
    match doc.get(obj, &prop)? {
        Some((Value::Object(ObjType::List), list_obj)) => {
            let length = doc.length(&list_obj);
            let mut result = Vec::new();
            for i in 0..length {
                match doc.get(&list_obj, i)? {
                    Some((Value::Scalar(s), _)) => {
                        if let Some(text) = s.to_str() {
                            result.push(text.to_string());
                        }
                    }
                    Some((Value::Object(ObjType::Text), text_obj)) => {
                        result.push(doc.text(&text_obj)?);
                    }
                    _ => {}
                }
            }
            Ok(result)
        }
        _ => Ok(Vec::new()),
    }
}

#[derive(Debug, Clone, Reconcile, Hydrate)]
pub struct Metadata {
    #[autosurgeon(hydrate = "hydrate_optional_timestamp")]
    pub createdAt: Option<i64>,
    #[autosurgeon(hydrate = "hydrate_optional_timestamp")]
    pub lastModified: Option<i64>,
    pub title: Option<autosurgeon::Text>,
}

#[derive(Debug, Clone, Reconcile, Hydrate)]
pub struct Doc {
    pub counter: i64,
    pub temperature: i64,
    pub darkMode: bool,
    pub notes: autosurgeon::Text,
    pub code: autosurgeon::Text,
    #[autosurgeon(hydrate = "hydrate_string_vec")]
    pub tags: Vec<String>,
    pub todos: Vec<TodoItem>,
    pub metadata: Metadata,
}

impl Doc {
    pub fn display(&self) {
        println!("\n📊 Autodash State:");
        println!("╭─────────────────────────────────────────╮");
        println!("│ 🔢 Counter: {:<28}│", self.counter);
        println!("│ 🌡️  Temperature: {}°C{:<22}│", self.temperature, "");
        println!(
            "│ 🌙 Dark Mode: {:<26}│",
            if self.darkMode { "ON" } else { "OFF" }
        );
        if self.notes.as_str().is_empty() {
            println!("│ 📝 Notes: (empty){:<22}│", "");
        } else {
            let notes_str = self.notes.as_str();
            let notes_preview = if notes_str.len() > 30 {
                format!("{}...", &notes_str[..27])
            } else {
                notes_str.to_string()
            };
            println!("│ 📝 Notes: {:<28}│", notes_preview);
        }
        if self.code.as_str().is_empty() {
            println!("│ 💻 Code: (empty){:<23}│", "");
        } else {
            let code_str = self.code.as_str();
            let code_lines = code_str.lines().count();
            let code_chars = code_str.chars().count();
            println!("│ 💻 Code: {} lines, {} chars{:<11}│", code_lines, code_chars, "");
        }
        println!("│ ✓  Todos: {:<28}│", self.todos.len());
        println!("│ 🏷️  Tags: {:<29}│", self.tags.len());
        if let Some(title) = &self.metadata.title {
            let title_str = title.as_str();
            let title_preview = if title_str.len() > 30 {
                format!("{}...", &title_str[..27])
            } else {
                title_str.to_string()
            };
            println!("│ 📄 Title: {:<28}│", title_preview);
        }
        println!("╰─────────────────────────────────────────╯");

        if !self.tags.is_empty() {
            println!("\n🏷️  Tags: {}", self.tags.join(", "));
        }

        if !self.todos.is_empty() {
            println!("\n✓ Todos:");
            for todo in &self.todos {
                let status = if todo.completed { "✓" } else { "○" };
                println!("  {} [{}] {}", status, todo.id.as_str(), todo.text.as_str());
            }
        }
    }

    pub fn display_field(&self, field: &str) {
        match field.to_lowercase().as_str() {
            "counter" => {
                println!("🔢 Counter: {}", self.counter);
            }
            "temperature" => {
                println!("🌡️  Temperature: {}°C", self.temperature);
            }
            "darkmode" | "dark_mode" => {
                println!("🌙 Dark Mode: {}", if self.darkMode { "ON" } else { "OFF" });
            }
            "notes" => {
                println!("📝 Notes:");
                if self.notes.as_str().is_empty() {
                    println!("  (empty)");
                } else {
                    println!("{}", self.notes.as_str());
                }
            }
            "code" => {
                println!("💻 Code:");
                if self.code.as_str().is_empty() {
                    println!("  (empty)");
                } else {
                    println!("{}", self.code.as_str());
                }
            }
            "todos" => {
                println!("✓ Todos ({}):", self.todos.len());
                if self.todos.is_empty() {
                    println!("  (none)");
                } else {
                    for todo in &self.todos {
                        let status = if todo.completed { "✓" } else { "○" };
                        println!("  {} [{}] {}", status, todo.id.as_str(), todo.text.as_str());
                    }
                }
            }
            "tags" => {
                println!("🏷️  Tags ({}):", self.tags.len());
                if self.tags.is_empty() {
                    println!("  (none)");
                } else {
                    println!("  {}", self.tags.join(", "));
                }
            }
            "metadata" => {
                println!("📄 Metadata:");
                if let Some(title) = &self.metadata.title {
                    println!("  Title: {}", title.as_str());
                }
                if let Some(created) = self.metadata.createdAt {
                    println!("  Created: {} ({})",
                        chrono::DateTime::from_timestamp_millis(created)
                            .map(|dt| dt.to_rfc3339())
                            .unwrap_or_else(|| "invalid".to_string()),
                        created
                    );
                }
                if let Some(modified) = self.metadata.lastModified {
                    println!("  Last Modified: {} ({})",
                        chrono::DateTime::from_timestamp_millis(modified)
                            .map(|dt| dt.to_rfc3339())
                            .unwrap_or_else(|| "invalid".to_string()),
                        modified
                    );
                }
            }
            _ => {
                println!("❌ Unknown field: {}", field);
                println!("Available fields: counter, temperature, darkMode, notes, code, todos, tags, metadata");
            }
        }
    }
}
