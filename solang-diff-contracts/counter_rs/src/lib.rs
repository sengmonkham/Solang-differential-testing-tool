#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Env, Symbol};

const COUNT_KEY: Symbol = symbol_short!("COUNT");

#[contract]
pub struct Counter;

#[contractimpl]
impl Counter {
    pub fn increment(env: Env) -> u64 {
        let mut count: u64 = env.storage().persistent().get(&COUNT_KEY).unwrap_or(0u64);
        count += 1;
        env.storage().persistent().set(&COUNT_KEY, &count);
        count
    }

    pub fn get(env: Env) -> u64 {
        env.storage().persistent().get(&COUNT_KEY).unwrap_or(0u64)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn test_counter() {
        let env = Env::default();
        let contract_id = env.register_contract(None, Counter);
        let client = CounterClient::new(&env, &contract_id);
        assert_eq!(client.increment(), 1);
        assert_eq!(client.increment(), 2);
        assert_eq!(client.get(), 2);
    }
}
