#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Env, String, Symbol, Vec, Address
};

// ============================================
// DATA TYPES / ENUMS
// ============================================

/// Categories of campus assets available for lending
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum ItemCategory {
    Electronics,  // Projectors, laptops, microphones
    Sports,       // Balls, rackets, mats
    Rooms,        // Halls, labs, meeting rooms
    Equipment,    // Toolkits, drawing tools, cameras
}

/// Current status of an inventory item
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum ItemStatus {
    Available,
    Borrowed,
    UnderMaintenance,
}

/// Status of a loan/borrowing transaction
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum LoanStatus {
    Active,
    Completed,
    Overdue,
}

// ============================================
// DATA STRUCTS
// ============================================

/// Represents a campus asset/item in the inventory
#[contracttype]
#[derive(Clone, Debug)]
pub struct Item {
    pub id: u64,
    pub name: String,
    pub category: ItemCategory,
    pub description: String,
    pub location: String,           // e.g. "Building A, Floor 2, Room 205"
    pub status: ItemStatus,
    pub total_quantity: u32,
    pub available_quantity: u32,
}

/// Represents a loan/borrowing transaction record
#[contracttype]
#[derive(Clone, Debug)]
pub struct Loan {
    pub id: u64,
    pub item_id: u64,
    pub borrower: Address,                  // Student/staff wallet address
    pub purpose: String,
    pub borrow_date: u64,                   // Unix timestamp
    pub planned_return_date: u64,           // Unix timestamp
    pub actual_return_date: u64,            // 0 = not yet returned
    pub status: LoanStatus,
    pub extended: bool,                     // Whether the loan has been extended
}

/// Overall contract statistics
#[contracttype]
#[derive(Clone, Debug)]
pub struct Statistics {
    pub total_item_types: u32,
    pub total_units: u32,
    pub available_units: u32,
    pub borrowed_units: u32,
    pub total_loans: u32,
    pub active_loans: u32,
    pub completed_loans: u32,
    pub overdue_loans: u32,
}

// ============================================
// STORAGE KEYS
// ============================================
const ITEMS_DATA: Symbol = symbol_short!("ITEMS");
const LOANS_DATA: Symbol = symbol_short!("LOANS");
const ITEM_COUNT: Symbol = symbol_short!("I_COUNT");
const LOAN_COUNT: Symbol = symbol_short!("L_COUNT");
const ADMIN: Symbol = symbol_short!("ADMIN");

// ============================================
// SMART CONTRACT
// ============================================
#[contract]
pub struct CampusLending;

#[contractimpl]
impl CampusLending {

    // ============================
    // INITIALIZATION
    // ============================

    /// Initialize the contract. Can only be called once.
    /// Sets the admin address and initializes counters to zero.
    pub fn initialize(env: Env, admin: Address) -> String {
        if env.storage().instance().has(&ADMIN) {
            return String::from_str(&env, "Error: Contract already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&ADMIN, &admin);
        env.storage().instance().set(&ITEM_COUNT, &0u64);
        env.storage().instance().set(&LOAN_COUNT, &0u64);
        String::from_str(&env, "Contract initialized successfully")
    }

    // ============================
    // INVENTORY MANAGEMENT (ADMIN)
    // ============================

    /// Add a new item to the campus inventory.
    /// Only the admin can perform this action.
    pub fn add_item(
        env: Env,
        admin: Address,
        name: String,
        category: ItemCategory,
        description: String,
        location: String,
        quantity: u32,
    ) -> String {
        Self::require_admin(&env, &admin);
        assert!(quantity > 0, "Quantity must be greater than 0");

        let mut items: Vec<Item> = env.storage().instance()
            .get(&ITEMS_DATA)
            .unwrap_or(Vec::new(&env));

        let count: u64 = env.storage().instance()
            .get(&ITEM_COUNT)
            .unwrap_or(0u64);

        let item = Item {
            id: count + 1,
            name,
            category,
            description,
            location,
            status: ItemStatus::Available,
            total_quantity: quantity,
            available_quantity: quantity,
        };

        items.push_back(item);
        env.storage().instance().set(&ITEMS_DATA, &items);
        env.storage().instance().set(&ITEM_COUNT, &(count + 1));

        String::from_str(&env, "Item added successfully")
    }

    /// Update the status of an item (e.g. mark as under maintenance).
    /// Only the admin can perform this action.
    pub fn update_item_status(
        env: Env,
        admin: Address,
        item_id: u64,
        new_status: ItemStatus,
    ) -> String {
        Self::require_admin(&env, &admin);

        let mut items: Vec<Item> = env.storage().instance()
            .get(&ITEMS_DATA)
            .unwrap_or(Vec::new(&env));

        for i in 0..items.len() {
            let mut item = items.get(i).unwrap();
            if item.id == item_id {
                item.status = new_status;
                items.set(i, item);
                env.storage().instance().set(&ITEMS_DATA, &items);
                return String::from_str(&env, "Item status updated successfully");
            }
        }

        String::from_str(&env, "Error: Item not found")
    }

    /// Add more units to an existing item in the inventory.
    /// Only the admin can perform this action.
    pub fn update_item_quantity(
        env: Env,
        admin: Address,
        item_id: u64,
        additional_quantity: u32,
    ) -> String {
        Self::require_admin(&env, &admin);

        let mut items: Vec<Item> = env.storage().instance()
            .get(&ITEMS_DATA)
            .unwrap_or(Vec::new(&env));

        for i in 0..items.len() {
            let mut item = items.get(i).unwrap();
            if item.id == item_id {
                item.total_quantity += additional_quantity;
                item.available_quantity += additional_quantity;
                if item.available_quantity > 0 {
                    item.status = ItemStatus::Available;
                }
                items.set(i, item);
                env.storage().instance().set(&ITEMS_DATA, &items);
                return String::from_str(&env, "Item quantity updated successfully");
            }
        }

        String::from_str(&env, "Error: Item not found")
    }

    /// Remove an item from the inventory.
    /// Only allowed if no units are currently borrowed.
    pub fn remove_item(
        env: Env,
        admin: Address,
        item_id: u64,
    ) -> String {
        Self::require_admin(&env, &admin);

        let mut items: Vec<Item> = env.storage().instance()
            .get(&ITEMS_DATA)
            .unwrap_or(Vec::new(&env));

        for i in 0..items.len() {
            let item = items.get(i).unwrap();
            if item.id == item_id {
                if item.available_quantity < item.total_quantity {
                    return String::from_str(&env, "Error: Some units are still being borrowed");
                }
                items.remove(i);
                env.storage().instance().set(&ITEMS_DATA, &items);
                return String::from_str(&env, "Item removed successfully");
            }
        }

        String::from_str(&env, "Error: Item not found")
    }

    // ============================
    // LOAN TRANSACTIONS
    // ============================

    /// Borrow an item from the campus inventory.
    /// The borrower must authenticate, specify the item, purpose, and duration (1-30 days).
    pub fn borrow_item(
        env: Env,
        borrower: Address,
        item_id: u64,
        purpose: String,
        duration_days: u64,
    ) -> String {
        borrower.require_auth();
        assert!(duration_days > 0 && duration_days <= 30, "Loan duration must be 1-30 days");

        let mut items: Vec<Item> = env.storage().instance()
            .get(&ITEMS_DATA)
            .unwrap_or(Vec::new(&env));

        // Find and validate the item
        let mut item_index: Option<u32> = None;
        for i in 0..items.len() {
            let b = items.get(i).unwrap();
            if b.id == item_id {
                if b.available_quantity == 0 {
                    return String::from_str(&env, "Error: Item is currently unavailable");
                }
                if b.status == ItemStatus::UnderMaintenance {
                    return String::from_str(&env, "Error: Item is under maintenance");
                }
                item_index = Some(i);
                break;
            }
        }

        let idx = match item_index {
            Some(i) => i,
            None => return String::from_str(&env, "Error: Item not found"),
        };

        // Decrease available stock
        let mut item = items.get(idx).unwrap();
        item.available_quantity -= 1;
        if item.available_quantity == 0 {
            item.status = ItemStatus::Borrowed;
        }
        items.set(idx, item);
        env.storage().instance().set(&ITEMS_DATA, &items);

        // Create loan record
        let mut loans: Vec<Loan> = env.storage().instance()
            .get(&LOANS_DATA)
            .unwrap_or(Vec::new(&env));

        let count: u64 = env.storage().instance()
            .get(&LOAN_COUNT)
            .unwrap_or(0u64);

        let now = env.ledger().timestamp();
        let one_day: u64 = 86400;

        let loan = Loan {
            id: count + 1,
            item_id,
            borrower,
            purpose,
            borrow_date: now,
            planned_return_date: now + (duration_days * one_day),
            actual_return_date: 0,
            status: LoanStatus::Active,
            extended: false,
        };

        loans.push_back(loan);
        env.storage().instance().set(&LOANS_DATA, &loans);
        env.storage().instance().set(&LOAN_COUNT, &(count + 1));

        String::from_str(&env, "Item borrowed successfully")
    }

    /// Return a borrowed item.
    /// Only the original borrower can return the item.
    /// If returned after the planned return date, the loan is marked as overdue.
    pub fn return_item(
        env: Env,
        borrower: Address,
        loan_id: u64,
    ) -> String {
        borrower.require_auth();

        let mut loans: Vec<Loan> = env.storage().instance()
            .get(&LOANS_DATA)
            .unwrap_or(Vec::new(&env));

        let mut items: Vec<Item> = env.storage().instance()
            .get(&ITEMS_DATA)
            .unwrap_or(Vec::new(&env));

        let now = env.ledger().timestamp();
        let mut target_item_id: u64 = 0;
        let mut found = false;

        // Update loan record
        for i in 0..loans.len() {
            let mut loan = loans.get(i).unwrap();
            if loan.id == loan_id {
                if loan.borrower != borrower {
                    return String::from_str(&env, "Error: Not the authorized borrower");
                }
                if loan.status != LoanStatus::Active {
                    return String::from_str(&env, "Error: Loan is already completed");
                }

                target_item_id = loan.item_id;
                loan.actual_return_date = now;

                if now > loan.planned_return_date {
                    loan.status = LoanStatus::Overdue;
                } else {
                    loan.status = LoanStatus::Completed;
                }

                loans.set(i, loan);
                found = true;
                break;
            }
        }

        if !found {
            return String::from_str(&env, "Error: Loan record not found");
        }

        // Restore item stock
        for i in 0..items.len() {
            let mut item = items.get(i).unwrap();
            if item.id == target_item_id {
                item.available_quantity += 1;
                if item.status == ItemStatus::Borrowed && item.available_quantity > 0 {
                    item.status = ItemStatus::Available;
                }
                items.set(i, item);
                break;
            }
        }

        env.storage().instance().set(&LOANS_DATA, &loans);
        env.storage().instance().set(&ITEMS_DATA, &items);

        String::from_str(&env, "Item returned successfully, thank you!")
    }

    /// Extend the loan duration (max 7 additional days, one-time only).
    /// Can only be extended if the loan is still active and not past the deadline.
    pub fn extend_loan(
        env: Env,
        borrower: Address,
        loan_id: u64,
        additional_days: u64,
    ) -> String {
        borrower.require_auth();
        assert!(additional_days > 0 && additional_days <= 7, "Extension is limited to 7 days max");

        let mut loans: Vec<Loan> = env.storage().instance()
            .get(&LOANS_DATA)
            .unwrap_or(Vec::new(&env));

        let now = env.ledger().timestamp();
        let one_day: u64 = 86400;

        for i in 0..loans.len() {
            let mut loan = loans.get(i).unwrap();
            if loan.id == loan_id {
                if loan.borrower != borrower {
                    return String::from_str(&env, "Error: Not the authorized borrower");
                }
                if loan.status != LoanStatus::Active {
                    return String::from_str(&env, "Error: Loan is not active");
                }
                if loan.extended {
                    return String::from_str(&env, "Error: Loan has already been extended once");
                }
                if now > loan.planned_return_date {
                    return String::from_str(&env, "Error: Loan has passed the deadline");
                }

                loan.planned_return_date += additional_days * one_day;
                loan.extended = true;
                loans.set(i, loan);
                env.storage().instance().set(&LOANS_DATA, &loans);
                return String::from_str(&env, "Loan extended successfully");
            }
        }

        String::from_str(&env, "Error: Loan record not found")
    }

    // ============================
    // QUERY / READ FUNCTIONS
    // ============================

    /// Get all items in the inventory
    pub fn get_all_items(env: Env) -> Vec<Item> {
        env.storage().instance()
            .get(&ITEMS_DATA)
            .unwrap_or(Vec::new(&env))
    }

    /// Get only available items (items with available_quantity > 0)
    pub fn get_available_items(env: Env) -> Vec<Item> {
        let all: Vec<Item> = env.storage().instance()
            .get(&ITEMS_DATA)
            .unwrap_or(Vec::new(&env));

        let mut result: Vec<Item> = Vec::new(&env);
        for i in 0..all.len() {
            let item = all.get(i).unwrap();
            if item.available_quantity > 0 {
                result.push_back(item);
            }
        }
        result
    }

    /// Get items filtered by category
    pub fn get_items_by_category(env: Env, category: ItemCategory) -> Vec<Item> {
        let all: Vec<Item> = env.storage().instance()
            .get(&ITEMS_DATA)
            .unwrap_or(Vec::new(&env));

        let mut result: Vec<Item> = Vec::new(&env);
        for i in 0..all.len() {
            let item = all.get(i).unwrap();
            if item.category == category {
                result.push_back(item);
            }
        }
        result
    }

    /// Get all loan records (admin view)
    pub fn get_all_loans(env: Env) -> Vec<Loan> {
        env.storage().instance()
            .get(&LOANS_DATA)
            .unwrap_or(Vec::new(&env))
    }

    /// Get loan history for a specific user
    pub fn get_loans_by_user(env: Env, borrower: Address) -> Vec<Loan> {
        let all: Vec<Loan> = env.storage().instance()
            .get(&LOANS_DATA)
            .unwrap_or(Vec::new(&env));

        let mut result: Vec<Loan> = Vec::new(&env);
        for i in 0..all.len() {
            let loan = all.get(i).unwrap();
            if loan.borrower == borrower {
                result.push_back(loan);
            }
        }
        result
    }

    /// Get all currently active loans
    pub fn get_active_loans(env: Env) -> Vec<Loan> {
        let all: Vec<Loan> = env.storage().instance()
            .get(&LOANS_DATA)
            .unwrap_or(Vec::new(&env));

        let mut result: Vec<Loan> = Vec::new(&env);
        for i in 0..all.len() {
            let loan = all.get(i).unwrap();
            if loan.status == LoanStatus::Active {
                result.push_back(loan);
            }
        }
        result
    }

    /// Get all overdue loans (past deadline or returned late)
    pub fn get_overdue_loans(env: Env) -> Vec<Loan> {
        let all: Vec<Loan> = env.storage().instance()
            .get(&LOANS_DATA)
            .unwrap_or(Vec::new(&env));

        let now = env.ledger().timestamp();
        let mut result: Vec<Loan> = Vec::new(&env);

        for i in 0..all.len() {
            let loan = all.get(i).unwrap();
            let active_overdue = loan.status == LoanStatus::Active
                && now > loan.planned_return_date;
            let already_overdue = loan.status == LoanStatus::Overdue;

            if active_overdue || already_overdue {
                result.push_back(loan);
            }
        }
        result
    }

    /// Get overall contract statistics (dashboard data)
    pub fn get_statistics(env: Env) -> Statistics {
        let items: Vec<Item> = env.storage().instance()
            .get(&ITEMS_DATA)
            .unwrap_or(Vec::new(&env));
        let loans: Vec<Loan> = env.storage().instance()
            .get(&LOANS_DATA)
            .unwrap_or(Vec::new(&env));

        let now = env.ledger().timestamp();

        let mut total_units: u32 = 0;
        let mut available_units: u32 = 0;
        for i in 0..items.len() {
            let item = items.get(i).unwrap();
            total_units += item.total_quantity;
            available_units += item.available_quantity;
        }

        let mut active: u32 = 0;
        let mut completed: u32 = 0;
        let mut overdue: u32 = 0;
        for i in 0..loans.len() {
            let loan = loans.get(i).unwrap();
            match loan.status {
                LoanStatus::Active => {
                    if now > loan.planned_return_date {
                        overdue += 1;
                    } else {
                        active += 1;
                    }
                },
                LoanStatus::Completed => completed += 1,
                LoanStatus::Overdue => overdue += 1,
            }
        }

        Statistics {
            total_item_types: items.len(),
            total_units,
            available_units,
            borrowed_units: total_units - available_units,
            total_loans: loans.len(),
            active_loans: active,
            completed_loans: completed,
            overdue_loans: overdue,
        }
    }

    // ============================
    // INTERNAL HELPERS
    // ============================

    /// Verify that the caller is the admin. Panics if not.
    fn require_admin(env: &Env, caller: &Address) {
        let admin: Address = env.storage().instance()
            .get(&ADMIN)
            .expect("Contract not initialized");
        assert!(*caller == admin, "Only admin can perform this action");
        caller.require_auth();
    }
}

mod test;
