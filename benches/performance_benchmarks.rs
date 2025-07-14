use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use claude_code_rust::config::{ConfigManager, ConfigFormat};
use claude_code_rust::fs::FileSystemManager;
use claude_code_rust::network::{MessageRequest, Message, MessageContent, ContentBlock};
use claude_code_rust::context::ContextManager;
use claude_code_rust::conversation::ConversationManager;
use tempfile::TempDir;
use std::time::Duration;
use tokio::runtime::Runtime;

fn bench_config_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("config_manager_creation", |b| {
        b.iter(|| {
            let _config_manager = ConfigManager::new();
        })
    });
    
    c.bench_function("config_file_creation", |b| {
        b.to_async(&rt).iter(|| async {
            let temp_dir = TempDir::new().unwrap();
            let config_path = temp_dir.path().join("bench_config.yaml");
            
            ConfigManager::create_example_config(&config_path, ConfigFormat::Yaml).await.unwrap();
        })
    });
}

fn bench_file_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let fs_manager = FileSystemManager::new();
    
    let mut group = c.benchmark_group("file_operations");
    
    for size in [1024, 10240, 102400].iter() {
        let content = "x".repeat(*size);
        
        group.bench_with_input(BenchmarkId::new("write_file", size), size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let temp_dir = TempDir::new().unwrap();
                let file_path = temp_dir.path().join("bench_file.txt");
                
                fs_manager.write_file(&file_path, &content).await.unwrap();
            })
        });
        
        group.bench_with_input(BenchmarkId::new("read_file", size), size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let temp_dir = TempDir::new().unwrap();
                let file_path = temp_dir.path().join("bench_file.txt");
                
                // 先写入文件
                fs_manager.write_file(&file_path, &content).await.unwrap();
                
                // 然后读取
                let _content = fs_manager.read_file(&file_path).await.unwrap();
            })
        });
    }
    
    group.finish();
}

fn bench_context_management(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_management");
    
    for message_count in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("add_messages", message_count), message_count, |b, &count| {
            b.iter(|| {
                let mut context_manager = ContextManager::new(8192);
                
                for i in 0..count {
                    let message = Message {
                        role: "user".to_string(),
                        content: MessageContent::Text(format!("Message {}", i)),
                    };
                    context_manager.add_message(black_box(message));
                }
            })
        });
        
        group.bench_with_input(BenchmarkId::new("get_messages", message_count), message_count, |b, &count| {
            b.iter_batched(
                || {
                    let mut context_manager = ContextManager::new(8192);
                    for i in 0..count {
                        let message = Message {
                            role: "user".to_string(),
                            content: MessageContent::Text(format!("Message {}", i)),
                        };
                        context_manager.add_message(message);
                    }
                    context_manager
                },
                |context_manager| {
                    let _messages = context_manager.get_messages();
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }
    
    group.finish();
}

fn bench_conversation_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("conversation_operations");
    
    group.bench_function("create_conversation", |b| {
        b.to_async(&rt).iter(|| async {
            let temp_dir = TempDir::new().unwrap();
            let mut conv_manager = ConversationManager::new(temp_dir.path().to_path_buf()).unwrap();
            
            let _conv_id = conv_manager.create_conversation(Some("Benchmark Conversation".to_string())).unwrap();
        })
    });
    
    group.bench_function("add_message", |b| {
        b.to_async(&rt).iter(|| async {
            let temp_dir = TempDir::new().unwrap();
            let mut conv_manager = ConversationManager::new(temp_dir.path().to_path_buf()).unwrap();
            
            let _conv_id = conv_manager.create_conversation(Some("Benchmark Conversation".to_string())).unwrap();
            let _msg_id = conv_manager.add_message("user", "Hello", None).unwrap();
        })
    });
    
    group.finish();
}

fn bench_json_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_operations");
    
    // 创建不同大小的消息请求
    for message_count in [1, 10, 100].iter() {
        let messages: Vec<Message> = (0..*message_count).map(|i| {
            Message {
                role: "user".to_string(),
                content: MessageContent::Blocks(vec![
                    ContentBlock::Text {
                        text: format!("This is message number {} with some content to make it realistic", i),
                    }
                ]),
            }
        }).collect();
        
        let request = MessageRequest {
            model: "claude-3-haiku-20240307".to_string(),
            max_tokens: 4096,
            messages,
            system: None,
            temperature: Some(0.7),
            top_p: Some(0.9),
            top_k: Some(40),
            stream: Some(false),
            tools: None,
            tool_choice: None,
            metadata: None,
            stop_sequences: None,
        };
        
        group.bench_with_input(BenchmarkId::new("serialize_request", message_count), &request, |b, req| {
            b.iter(|| {
                let _json = serde_json::to_string(black_box(req)).unwrap();
            })
        });
        
        group.bench_with_input(BenchmarkId::new("serialize_pretty", message_count), &request, |b, req| {
            b.iter(|| {
                let _json = serde_json::to_string_pretty(black_box(req)).unwrap();
            })
        });
    }
    
    group.finish();
}

fn bench_string_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_operations");
    
    for size in [100, 1000, 10000].iter() {
        let text = "Hello, World! ".repeat(*size);
        
        group.bench_with_input(BenchmarkId::new("string_split", size), &text, |b, text| {
            b.iter(|| {
                let _words: Vec<&str> = text.split_whitespace().collect();
            })
        });
        
        group.bench_with_input(BenchmarkId::new("string_replace", size), &text, |b, text| {
            b.iter(|| {
                let _replaced = text.replace("Hello", "Hi");
            })
        });
        
        group.bench_with_input(BenchmarkId::new("string_to_lowercase", size), &text, |b, text| {
            b.iter(|| {
                let _lower = text.to_lowercase();
            })
        });
    }
    
    group.finish();
}

fn bench_memory_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation");
    
    for size in [1000, 10000, 100000].iter() {
        group.bench_with_input(BenchmarkId::new("vec_allocation", size), size, |b, &size| {
            b.iter(|| {
                let _vec: Vec<u8> = vec![0; size];
            })
        });
        
        group.bench_with_input(BenchmarkId::new("string_allocation", size), size, |b, &size| {
            b.iter(|| {
                let _string = String::with_capacity(size);
            })
        });
        
        group.bench_with_input(BenchmarkId::new("vec_push", size), size, |b, &size| {
            b.iter(|| {
                let mut vec = Vec::new();
                for i in 0..size {
                    vec.push(black_box(i));
                }
            })
        });
    }
    
    group.finish();
}

fn bench_concurrent_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("concurrent_operations");
    group.measurement_time(Duration::from_secs(10));
    
    for thread_count in [1, 2, 4, 8].iter() {
        group.bench_with_input(BenchmarkId::new("parallel_config_creation", thread_count), thread_count, |b, &count| {
            b.to_async(&rt).iter(|| async move {
                let tasks: Vec<_> = (0..count).map(|_| {
                    tokio::spawn(async {
                        let _config_manager = ConfigManager::new();
                    })
                }).collect();
                
                for task in tasks {
                    task.await.unwrap();
                }
            })
        });
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_config_operations,
    bench_file_operations,
    bench_context_management,
    bench_conversation_operations,
    bench_json_operations,
    bench_string_operations,
    bench_memory_allocation,
    bench_concurrent_operations
);

criterion_main!(benches);
