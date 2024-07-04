use std::collections::{BTreeMap, HashMap};

type AccountId = u64;
type Balance = u128;

struct Bank {
  account_id_next: AccountId,
  balances: BTreeMap<AccountId, Balance>,
}

impl Bank {
  pub const TREASURY: AccountId = 0;

  pub fn new() -> Bank {
    let mut result = Bank {
      account_id_next: 0,
      balances: BTreeMap::new(),
    };
    result.create_treasury(1_000_000);
    result
  }

  fn create_treasury(&mut self, treasury_balance: Balance) {
    let treasury_account = self.create_account();
    assert!(treasury_account == Bank::TREASURY);
    assert!(self.balances.len() == 1);
    self.balances.insert(treasury_account, treasury_balance);
  }

  pub fn create_account(&mut self) -> AccountId {
    let result = self.account_id_next;
    self.account_id_next += 1;
    self.balances.insert(result, 0);
    result
  }

  pub fn delete_account(&mut self, account_id: AccountId) {
    assert!(account_id != Bank::TREASURY);
    self.balances.remove(&account_id);
  }

  pub fn get_accounts(&self) -> impl ExactSizeIterator<Item = AccountId> + '_ {
    self.balances.keys().copied()
  }

  pub fn lookup_balance(&self, account_id: AccountId) -> Balance {
    *self.balances.get(&account_id).unwrap()
  }

  pub fn transfer(&mut self, dr: AccountId, cr: AccountId, amount: Balance) {
    if dr == cr {
      return;
    }
    let dr_balance = *self.balances.get(&dr).unwrap();
    let cr_balance = *self.balances.get(&cr).unwrap();
    match (
      dr_balance.checked_sub(amount),
      cr_balance.checked_add(amount),
    ) {
      (Some(dr_balance_new), Some(cr_balance_new)) => {
        self.balances.insert(dr, dr_balance_new);
        self.balances.insert(cr, cr_balance_new);
      }
      _ => (),
    }
  }
}

#[test]
fn test_bank() {
  arbtest::arbtest(|rng| {
    let mut bank = Bank::new();

    let balance_total_initial: Balance =
      bank.get_accounts().map(|it| bank.lookup_balance(it)).sum();

    let mut accounts_created_count = 0;
    let mut transfers_created_count = 0;
    while !rng.is_empty() {
      match *rng.choose(&["create", "delete", "transfer"])? {
        "create" => {
          bank.create_account();
          accounts_created_count += 1;
        }
        "delete" => {
          let account = rng.choose_iter(bank.get_accounts())?;
          if account != Bank::TREASURY {
            bank.delete_account(account);
          }
        }
        "transfer" => {
          let dr = rng.choose_iter(bank.get_accounts())?;
          let cr = rng.choose_iter(bank.get_accounts())?;
          let amount = rng.int_in_range(0..=100)?;
          bank.transfer(dr, cr, amount);
          transfers_created_count += 1;
        }
        _ => unreachable!(),
      }
      let balance_total: Balance = bank.get_accounts().map(|it| bank.lookup_balance(it)).sum();
      if (balance_total != balance_total_initial) {
        eprint!(
          "accounts created {}, transfers created {} ",
          accounts_created_count, transfers_created_count,
        );
        panic!();
      }
    }
    Ok(())
  })
  .seed(0xff9d5f7f00000020)
  .minimize();
  // accounts created 4, transfers created 4 seed 0xff9d5f7f00000020, seed size 32, search time 103.00ns
  // accounts created 1, transfers created 1 seed 0x61955ad200000010, seed size 16, search time 351.87µs
  // accounts created 1, transfers created 1 seed 0xd5bf1b3a00000008, seed size 8, search time 1.08ms
  // accounts created 1, transfers created 1 seed 0x62b3860600000007, seed size 7, search time 21.60ms
  // accounts created 1, transfers created 1 seed 0x68a8d5af00000006, seed size 6, search time 21.70ms
  // accounts created 1, transfers created 1 seed 0x43e1e68400000005, seed size 5, search time 21.76ms
  // minimized
  // seed 0x43e1e68400000005, seed size 5, search time 100.00ms
}
