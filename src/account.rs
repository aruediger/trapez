#[derive(Debug, PartialEq)]
struct Account {
    available: i64,
    held: i64,
    locked: bool,
}

impl Account {
    fn new() -> Account {
        Account {
            available: 0,
            held: 0,
            locked: false,
        }
    }

    fn total(self) -> i64 {
        self.available + self.held
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create() {
        let account = Account::new();
        assert_eq!(
            account,
            Account {
                available: 0,
                held: 0,
                locked: false
            }
        );
        assert_eq!(account.total(), 0)
    }
}
