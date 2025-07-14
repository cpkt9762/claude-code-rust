use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::collections::HashMap;
use std::time::Duration;

/// 简单的配置管理基准测试
fn config_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_management");
    
    // JSON 解析基准测试
    group.bench_function("json_parse", |b| {
        let json_content = r#"
        {
            "api": {
                "anthropic_api_key": "test-key",
                "base_url": "https://api.anthropic.com",
                "default_model": "claude-3-haiku-20240307",
                "max_tokens": 4096,
                "temperature": 0.7
            },
            "logging": {
                "level": "info",
                "console": true,
                "structured": false
            }
        }
        "#;
        
        b.iter(|| {
            let _config: Result<serde_json::Value, _> = 
                serde_json::from_str(black_box(json_content));
        });
    });

    // HashMap 操作基准测试
    group.bench_function("hashmap_operations", |b| {
        b.iter(|| {
            let mut map = HashMap::new();
            for i in 0..black_box(1000) {
                map.insert(format!("key_{}", i), i);
            }
            
            for i in 0..black_box(500) {
                let _value = map.get(&format!("key_{}", i));
            }
            
            black_box(map);
        });
    });

    group.finish();
}

/// 字符串处理基准测试
fn string_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_processing");
    
    let test_string = "Hello, World! This is a test string for benchmarking.".repeat(10);
    
    // 字符串连接基准测试
    group.bench_function("string_concatenation", |b| {
        b.iter(|| {
            let mut result = String::new();
            for _ in 0..black_box(100) {
                result.push_str(black_box(&test_string));
            }
            black_box(result);
        });
    });

    // 字符串搜索基准测试
    group.bench_function("string_search", |b| {
        b.iter(|| {
            let _found = test_string.contains(black_box("test"));
        });
    });

    // 字符串分割基准测试
    group.bench_function("string_split", |b| {
        b.iter(|| {
            let parts: Vec<&str> = test_string.split(black_box(" ")).collect();
            black_box(parts);
        });
    });

    group.finish();
}

/// 数据结构基准测试
fn data_structure_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_structures");
    
    // 向量操作基准测试
    for size in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        
        group.bench_with_input(BenchmarkId::new("vec_push", size), size, |b, &size| {
            b.iter(|| {
                let mut vec = Vec::new();
                for i in 0..black_box(size) {
                    vec.push(i);
                }
                black_box(vec);
            });
        });
        
        group.bench_with_input(BenchmarkId::new("vec_with_capacity", size), size, |b, &size| {
            b.iter(|| {
                let mut vec = Vec::with_capacity(size);
                for i in 0..black_box(size) {
                    vec.push(i);
                }
                black_box(vec);
            });
        });
    }

    group.finish();
}

/// 序列化基准测试
fn serialization_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");
    
    #[derive(serde::Serialize, serde::Deserialize, Clone)]
    struct TestData {
        id: u64,
        name: String,
        email: String,
        metadata: HashMap<String, String>,
    }

    let test_data = TestData {
        id: 12345,
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
        metadata: {
            let mut map = HashMap::new();
            map.insert("key1".to_string(), "value1".to_string());
            map.insert("key2".to_string(), "value2".to_string());
            map.insert("key3".to_string(), "value3".to_string());
            map
        },
    };

    // JSON 序列化基准测试
    group.bench_function("json_serialize", |b| {
        b.iter(|| {
            let _json = serde_json::to_string(black_box(&test_data)).unwrap();
        });
    });

    // JSON 反序列化基准测试
    group.bench_function("json_deserialize", |b| {
        let json = serde_json::to_string(&test_data).unwrap();
        b.iter(|| {
            let _data: TestData = serde_json::from_str(black_box(&json)).unwrap();
        });
    });

    // Bincode 序列化基准测试
    group.bench_function("bincode_serialize", |b| {
        b.iter(|| {
            let _bytes = bincode::serialize(black_box(&test_data)).unwrap();
        });
    });

    // Bincode 反序列化基准测试
    group.bench_function("bincode_deserialize", |b| {
        let bytes = bincode::serialize(&test_data).unwrap();
        b.iter(|| {
            let _data: TestData = bincode::deserialize(black_box(&bytes)).unwrap();
        });
    });

    group.finish();
}

/// 算法基准测试
fn algorithm_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("algorithms");
    
    // 排序基准测试
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("sort", size), size, |b, &size| {
            let mut data: Vec<i32> = (0..size).rev().collect();
            b.iter(|| {
                let mut data_copy = data.clone();
                data_copy.sort();
                black_box(data_copy);
            });
        });
    }

    // 查找基准测试
    group.bench_function("binary_search", |b| {
        let data: Vec<i32> = (0..10000).collect();
        b.iter(|| {
            let _result = data.binary_search(&black_box(5000));
        });
    });

    // 哈希计算基准测试
    group.bench_function("hash_calculation", |b| {
        let data = "Hello, World! This is a test string for hashing.".repeat(100);
        b.iter(|| {
            let _hash = md5::compute(black_box(data.as_bytes()));
        });
    });

    group.finish();
}

/// 内存分配基准测试
fn memory_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation");
    
    // 小对象分配基准测试
    group.bench_function("small_allocations", |b| {
        b.iter(|| {
            let mut boxes = Vec::new();
            for i in 0..black_box(1000) {
                boxes.push(Box::new(i));
            }
            black_box(boxes);
        });
    });

    // 大对象分配基准测试
    group.bench_function("large_allocations", |b| {
        b.iter(|| {
            let mut vecs = Vec::new();
            for _ in 0..black_box(100) {
                vecs.push(vec![0u8; 1024]);
            }
            black_box(vecs);
        });
    });

    // 字符串分配基准测试
    group.bench_function("string_allocations", |b| {
        b.iter(|| {
            let mut strings = Vec::new();
            for i in 0..black_box(1000) {
                strings.push(format!("String number {}", i));
            }
            black_box(strings);
        });
    });

    group.finish();
}

/// 并发模拟基准测试
fn concurrency_simulation_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrency_simulation");
    
    // 模拟并发工作负载
    group.bench_function("simulated_work", |b| {
        b.iter(|| {
            let mut results = Vec::new();
            for i in 0..black_box(100) {
                // 模拟一些计算工作
                let result = (0..100).map(|x| x * i).sum::<usize>();
                results.push(result);
            }
            black_box(results);
        });
    });

    // 模拟数据处理管道
    group.bench_function("data_pipeline", |b| {
        let input_data: Vec<i32> = (0..1000).collect();
        b.iter(|| {
            let result: Vec<i32> = input_data
                .iter()
                .map(|x| x * 2)
                .filter(|x| *x % 3 == 0)
                .map(|x| x + 1)
                .collect();
            black_box(result);
        });
    });

    group.finish();
}

/// 缓存模拟基准测试
fn cache_simulation_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_simulation");
    
    // 模拟 LRU 缓存
    group.bench_function("lru_cache_simulation", |b| {
        use std::collections::HashMap;
        
        b.iter(|| {
            let mut cache = HashMap::new();
            let mut access_order = Vec::new();
            let cache_size = 100;
            
            for i in 0..black_box(1000) {
                let key = i % 150; // 模拟缓存命中和未命中
                
                if cache.contains_key(&key) {
                    // 缓存命中，更新访问顺序
                    access_order.retain(|&x| x != key);
                    access_order.push(key);
                } else {
                    // 缓存未命中，添加新条目
                    if cache.len() >= cache_size {
                        // 移除最久未使用的条目
                        if let Some(lru_key) = access_order.first().copied() {
                            cache.remove(&lru_key);
                            access_order.remove(0);
                        }
                    }
                    cache.insert(key, format!("value_{}", key));
                    access_order.push(key);
                }
            }
            
            black_box((cache, access_order));
        });
    });

    group.finish();
}

// 基准测试组定义
criterion_group!(
    benches,
    config_benchmarks,
    string_benchmarks,
    data_structure_benchmarks,
    serialization_benchmarks,
    algorithm_benchmarks,
    memory_benchmarks,
    concurrency_simulation_benchmarks,
    cache_simulation_benchmarks
);

criterion_main!(benches);
