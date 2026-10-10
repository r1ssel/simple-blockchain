#![allow(dead_code, unused_variables, unused_imports)]

use serde::{Serilize, Deserialize};
use sha2::{Sha256, Digest};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Transaction {
    from: String,
    to: String,
    amount: u64,
}

impl Transaction {
    fn new(from: &str, to: &str, amount: u64) -> Self {
        Transaction {
            from.to_string(),
            to.to_string(),
            amount,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Block {
    index: u64,
    timestamp: u128,
    transaction: Vec<Transaction>,
    prev_hash: String,
    nonce: u64,
    hash: String,
}

impl Block {
    fn calculate_hash(&self) -> String {
        let txs = serde_json::to_string(&self.transaction).unwrap();
        let input = format!(
            "{}{}{}{}{}",
            self.index, self.timestamp, self.txs, self.prev_hash, self.nonce
        );
        // println!("{}", input);
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        hex::encode(hasher.finalize())    
    }
    
    fn mine(&mut self, difficulty: usize) {
        let target = "0".repeat(difficulty);
        loop {
            self.hash = self.calculate_hash();
            if self.hash.starts_with(&target) {
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
            timestamp: now(),
            transaction: vec![],
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

    fn add_tranaction(&mut self, tx: Transaction) {
        println!{"Транзакция добавлена в mempool: {} -> {}({})", tx.from, tx.to, tx.amount};
        self.mempool.push(tx);
    }
    
    fn add_block(&mut self, data: String) {
        let prev = self.chain.last().unwrap();
        let mut block = Block {
            index: prev.index + 1,
            timestamp: now(),
            data,
            prev_hash: prev.hash.clone(),
            nonce: 0,
            hash: String::new(),
        };
        block.mine(self.difficulty);
        println!("Блок №{} найден: {}", block.index, block.hash);
        self.chain.push(block);
    }

    

    fn is_valid(&self) -> bool {
        for i in 1..self.chain.len() {
            let current = &self.chain[i];
            let prev = &self.chain[i - 1];

            if current.hash != current.calculate_hash() {
                return false;
            }
        }
        true
    }
}

fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

fn main() {   
    let mut bc = Blockchain::new(4);

    println!("Майним блоки...");
    bc.add_block("Адиса -> Боб: 10 монет".to_string());
    bc.add_block("Боб -> Чарли: 5 монет".to_string()); 
    bc.add_block("Чарли -> Алиса: 10 монет".to_string());
    println!("\nЦепочка валидна: {}", bc.is_valid());

    // Ломаем цепочку: меняем данные во втором блоке
    println!("\n--- Подделываем данные в блоке #1 ---");
    bc.chain[1].data = "Алиса -> Боб: 1000000 монет".to_string();
    println!("Цепочка валидна после подделки: {}", bc.is_valid()); 
}
