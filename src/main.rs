#![allow(dead_code, unused_variables, unused_imports)]

use serde::{Serialize, Deserialize};
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
            from: from.to_string(),
            to: to.to_string(),
            amount: amount,
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
            self.index, self.timestamp, txs, self.prev_hash, self.nonce
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

    fn add_transaction(&mut self, tx: Transaction) {
        println!{"Транзакция добавлена в mempool: {} -> {}({})", tx.from, tx.to, tx.amount};
        self.mempool.push(tx);
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
            transaction: txs,
            prev_hash: prev.hash.clone(),
            nonce: 0,
            hash: String::new(),
        };
        block.mine(self.difficulty);
        println!("Блок #{} замайнен: {} ({} транзакций)",
                 block.index, block.hash, block.transaction.len());
        self.chain.push(block);
    }

    fn is_valid(&self) -> bool {
        for i in 1..self.chain.len() {
            let current = &self.chain[i];
            let prev = &self.chain[i - 1];

            if current.hash != current.calculate_hash() {
                return false;
            }
            
            if current.prev_hash != prev.hash {
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

    // Добавляем транзакции в mempool
    bc.add_transaction(Transaction::new("Алиса", "Боб", 10));
    bc.add_transaction(Transaction::new("Боб", "Чарли", 5));
    bc.add_transaction(Transaction::new("Чарли", "Алиса", 2));

    // Майним блок — все три транзакции попадут в один блок
    bc.mine_block();

    // Ещё транзакции
    bc.add_transaction(Transaction::new("Алиса", "Чарли", 1));
    bc.mine_block();

    println!("\nЦепочка валидна: {}", bc.is_valid());
    println!("Блоков: {}", bc.chain.len());
    for b in &bc.chain {
        println!("  Блок #{}: {} транзакций, hash={}", b.index, b.transaction.len(), &b.hash[..16]);
    }
}
