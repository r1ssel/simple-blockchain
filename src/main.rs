#![allow(dead_code, unused_variables, unused_imports)]

use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::thread::sleep;
use std::io::{self, Write};
use tokio::sync::Mutex;
use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use rand::rngs::OsRng;
use base64::{engine::general_purpose, Engine as _};


#[derive(Debug, Clone, Serialize, Deserialize)]
struct Transaction {
    from: String,
    to: String,
    amount: u64,
    pub_key: String,
    signature: String,
    timestamp: u128,
}

impl Transaction {
    fn new_signed(signing_key: &SigningKey, to: &str, amount: u64) -> Self {
        let from = general_purpose::STANDARD.encode(signing_key.verifying_key().as_bytes());
        let timestamp = now();

        let msg = format!("{}{}{}{}", from, to, amount, timestamp);
        let signature = signing_key.sign(msg.as_bytes());
        let signature_b64 = general_purpose::STANDARD.encode(signature.to_bytes());
        let pub_key_b64 = general_purpose::STANDARD.encode(signing_key.verifying_key().as_bytes());

        Transaction {
            from,
            to: to.to_string(),
            amount,
            pub_key: pub_key_b64,
            signature: signature_b64,
            timestamp,
        }
    }

    fn verify(&self) -> bool {
        let pub_key_bytes = match general_purpose::STANDARD.decode(&self.pub_key) {
            Ok(b) => b,
            Err(_) => return false,
        };

        let pub_key = match VerifyingKey::from_bytes(&pub_key_bytes.try_into().unwrap_or([0u8; 32])) {
            Ok(k) => k,
            Err(_) => return false,
        };

        let sig_bytes = match general_purpose::STANDARD.decode(&self.signature) {
            Ok(b) => b,
            Err(_) => return false,
        };

        let signature = match Signature::from_slice(&sig_bytes) {
           Ok(s) => s,
           Err(_) => return false,
        };

        let msg = format!("{}{}{}{}", self.from, self.to, self.amount, self.timestamp);
        pub_key.verify(msg.as_bytes(), &signature).is_ok()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Block {
    index: u64,
    timestamp: u128,
    transactions: Vec<Transaction>,
    prev_hash: String,
    nonce: u64,
    hash: String,
}

impl Block {
    fn calculate_hash(&self) -> String {
        let txs = serde_json::to_string(&self.transactions).unwrap();
        let input = format!(
            "{}{}{}{}{}",
            self.index, self.timestamp, txs, self.prev_hash, self.nonce
        );
        // println!("{}", input);
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        hex::encode(hasher.finalize())    
    }
    
    fn mine(&mut self, difficulty: usize) {
        let target = "0".repeat(difficulty);
        let start = Instant::now();
        let mut attempts: u64 = 0;

        // Ожидаемое число попыток: 16^difficulty
        let expected: f64 = (16_f64.powi(difficulty as i32))*10.0;
        let bar_width = 40;

        loop {
            self.hash = self.calculate_hash();
            attempts += 1;

            // Считаем прогресс как долю от ожидаемого числа попыток
            let progress = (attempts as f64 / expected).min(1.0);
            let filled = (progress * bar_width as f64) as usize;

            let bar = format!(
                "[{}{}] {:.2}% | попыток: {}",
                "█".repeat(filled),
                "░".repeat(bar_width - filled),
                progress * 100.0,
                attempts
            );

            print!("\r\x1b[2K{}", bar);
            io::stdout().flush().unwrap();

            if self.hash.starts_with(&target) {
                let elapsed = start.elapsed();
                print!("\r\x1b[2K");
                println!("  Найдено за {} попыток", attempts);
                println!("  Время: {:.2?}", elapsed);
                break;
            }

            self.nonce += 1;
        }
    } 
}

struct Blockchain {
    chain: Vec<Block>,
    mempool: Vec<Transaction>,
    difficulty: usize,
}

impl Blockchain {
    fn create_genesis_block(&self) -> Block {
        let mut block = Block {
            index: 0,
            timestamp: 0,
            transactions: vec![],
            prev_hash: "0".repeat(64),
            nonce: 0,
            hash: String::new(),
        };
        block.mine(self.difficulty);
        block
    }

    fn new(difficulty: usize) -> Self {
        let mut bc = Blockchain{
            chain: Vec::new(),
            mempool: Vec::new(),
            difficulty,
        };
        bc.chain.push(bc.create_genesis_block());
        bc
    }

    fn add_transaction(&mut self, tx: Transaction) -> bool {
        if !tx.verify() {
            println!("Транзакция отклонена: неверная подпись");
            return false;
        }
        let from_short = &tx.from[..tx.from.len().min(8)];
        let to_short = &tx.to[..tx.to.len().min(8)];
        println!("Транзакция добавлена в mempool: {} -> {} ({})", from_short, to_short, tx.amount); 
        self.mempool.push(tx);
        return true
    }

    fn mine_block(&mut self) {
        if self.mempool.is_empty() {
            println!("Mempool пуст");
            return;
        }
        
        let prev = self.chain.last().unwrap();
        let txs = std::mem::take(&mut self.mempool);
        
        let mut block = Block {
            index: prev.index + 1,
            timestamp: now(),
            transactions: txs,
            prev_hash: prev.hash.clone(),
            nonce: 0,
            hash: String::new(),
        };
        block.mine(self.difficulty);
        println!("Блок #{} замайнен: {} ({} транзакций)",
                 block.index, block.hash, block.transactions.len());
        self.chain.push(block);
    } 
}

fn is_chain_valid(chain: &Vec<Block>) -> bool {
    for i in 1..chain.len() {
        let current = &chain[i];
        let prev = &chain[i - 1];

        if current.hash != current.calculate_hash() {
            println!("Блок #{}: хэш не совпадает", current.index);
            return false;
        }

        if current.prev_hash != prev.hash {
            println!("Блок #{}: prev_hash не совпадает", current.index);
            return false;
        }
    }
    true
}

fn serialize_chain(chain: &Vec<Block>) -> String {
    serde_json::to_string(chain).unwrap() + "\n"
}

async fn handle_peer(mut stream: TcpStream, bc: Arc<Mutex<Blockchain>>) {
    let mut buf = vec![0u8; 65536];
    loop {
        match stream.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => {
                let msg = String::from_utf8_lossy(&buf[..n]).to_string();
                //println!("Получено {} байт", n);
                if let Ok(incoming_chain) = serde_json::from_str::<Vec<Block>>(&msg) {
                    //println!("Распарсено {} блоков", incoming_chain.len());
                    let mut local = bc.lock().await;
                    //println!("Локально {} блоков", local.chain.len());
                    if incoming_chain.len() > local.chain.len() && is_chain_valid(&incoming_chain) {
                        //println!("Принятая цепочка длиннее локальной");
                        if is_chain_valid(&incoming_chain) {
                            println!("Принята цепочка от соседа: {} блоков", incoming_chain.len());
                                for block in &incoming_chain {
                                    println!(
                                        "  Блок #{}: {} транзакций, hash={}",
                                        block.index,
                                        block.transactions.len(),
                                        &block.hash[..16]
                                        );
                                }
                            local.chain = incoming_chain;
                        } else {
                            println!("Цепочка невалидна");
                        }
                    }
                    else {
                        println!("Цепочка от соседа короче");
                    }
                } 
            }
            Err(_) => break,
        }
    }
}

async fn broadcast_chain(peer_addr: &str, chain: &Vec<Block>) {
    if let Ok(mut stream) = TcpStream::connect(peer_addr).await {
        let msg = serialize_chain(chain);
        let _ = stream.write_all(msg.as_bytes()).await;
        let _ = stream.shutdown().await;
    }
}

fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let my_port = &args[1];
    let peer_addr = &args[2];

    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let my_address = general_purpose::STANDARD.encode(signing_key.verifying_key().as_bytes());
    
    println!("Мой адрес: {}", my_address);
    println!("(первые 16 символов: {})", &my_address[..16]);

    let bc = Arc::new(Mutex::new(Blockchain::new(6)));
    
    let listener = TcpListener::bind(format!("127.0.0.1:{}", my_port)).await.unwrap();
    println!("Узел слушает на порту {}", my_port);

    let bc_clone = bc.clone();
    tokio::spawn(async move {
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let bc_inner = bc_clone.clone();
                tokio::spawn(handle_peer(stream, bc_inner));
            }
        }
    });

    loop {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        let cmd = input.trim();

        if cmd == "mine" {
            {
                let mut local = bc.lock().await;
                if local.mempool.is_empty() {
                    println!("Mempool пуст!");
                    continue;
                }
                local.mine_block();
            }
            let local = bc.lock().await;
            broadcast_chain(peer_addr, &local.chain).await;
            println!("Цепочка отправлена соседу {}", peer_addr);
        } 
        else if cmd.starts_with("tx ") {
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            if parts.len() == 3 {
                let to = parts[1];
                let amount = parts[2].parse().unwrap();
                let tx = Transaction::new_signed(&signing_key, to, amount);
                let mut local = bc.lock().await;
                local.add_transaction(tx);
            } else {
                println!("Формат: tx <кому> <сумма>");
            }
        } 
        else if cmd == "show" {
            let local = bc.lock().await;
            println!("Блоков: {}", local.chain.len());
            for b in &local.chain {
                println!("  #{}: {} tx, hash={}", b.index, b.transactions.len(), &b.hash[..16]);
                for t in &b.transactions {
                    println!("   {} -> {}: {}", t.from, t.to, t.amount);
                }
            }
        } 
        else if cmd == "chain" {
            let local = bc.lock().await;
            let mut count: u64 = 1;
            for b in &local.chain {  
                for t in &b.transactions {
                    println!(" tx#{} {} -> {} {}\n", count, t.from, t.to, t.amount);
                    count+=1;
                }
            }
        }
    }
}
