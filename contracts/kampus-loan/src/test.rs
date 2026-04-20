#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::{Address as _, Ledger, LedgerInfo}, Env, Address};

fn setup() -> (Env, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set(LedgerInfo {
        timestamp: 1_700_000_000,
        ..Default::default()
    });
    let admin = Address::generate(&env);
    let student = Address::generate(&env);
    (env, admin, student)
}

fn init_contract(env: &Env, admin: &Address) -> CampusLendingClient {
    let contract_id = env.register_contract(None, CampusLending);
    let client = CampusLendingClient::new(env, &contract_id);
    client.initialize(admin);
    client
}

fn add_sample_items(env: &Env, client: &CampusLendingClient, admin: &Address) {
    client.add_item(
        admin,
        &String::from_str(env, "Epson EB-X51 Projector"),
        &ItemCategory::Electronics,
        &String::from_str(env, "3800 lumen projector for presentations"),
        &String::from_str(env, "Building A, Floor 1, AV Storage"),
        &3u32,
    );
    client.add_item(
        admin,
        &String::from_str(env, "Basketball"),
        &ItemCategory::Sports,
        &String::from_str(env, "Size 7 Spalding basketball"),
        &String::from_str(env, "Sports Storage, Building C"),
        &5u32,
    );
    client.add_item(
        admin,
        &String::from_str(env, "Multi-Purpose Hall"),
        &ItemCategory::Rooms,
        &String::from_str(env, "Capacity 200, AC, sound system"),
        &String::from_str(env, "Main Building, Floor 1"),
        &1u32,
    );
}

// ========================
// INITIALIZATION TESTS
// ========================

#[test]
fn test_initialize_success() {
    let (env, admin, _) = setup();
    let client = init_contract(&env, &admin);
    let items = client.get_all_items();
    assert_eq!(items.len(), 0);
}

#[test]
fn test_initialize_twice_fails() {
    let (env, admin, _) = setup();
    let client = init_contract(&env, &admin);
    let result = client.initialize(&admin);
    assert!(result.to_string().contains("Error"));
}

// ========================
// ADD ITEM TESTS
// ========================

#[test]
fn test_add_item_success() {
    let (env, admin, _) = setup();
    let client = init_contract(&env, &admin);
    add_sample_items(&env, &client, &admin);

    let items = client.get_all_items();
    assert_eq!(items.len(), 3);
    assert_eq!(items.get(0).unwrap().total_quantity, 3);
    assert_eq!(items.get(0).unwrap().available_quantity, 3);
}

#[test]
fn test_item_id_auto_increment() {
    let (env, admin, _) = setup();
    let client = init_contract(&env, &admin);
    add_sample_items(&env, &client, &admin);

    let items = client.get_all_items();
    assert_eq!(items.get(0).unwrap().id, 1);
    assert_eq!(items.get(1).unwrap().id, 2);
    assert_eq!(items.get(2).unwrap().id, 3);
}

// ========================
// BORROW ITEM TESTS
// ========================

#[test]
fn test_borrow_item_success() {
    let (env, admin, student) = setup();
    let client = init_contract(&env, &admin);
    add_sample_items(&env, &client, &admin);

    let result = client.borrow_item(
        &student,
        &1u64,
        &String::from_str(&env, "Final thesis presentation"),
        &3u64,
    );
    assert!(result.to_string().contains("successfully"));

    let items = client.get_all_items();
    assert_eq!(items.get(0).unwrap().available_quantity, 2);
}

#[test]
fn test_borrow_out_of_stock_fails() {
    let (env, admin, _) = setup();
    let client = init_contract(&env, &admin);

    client.add_item(
        &admin,
        &String::from_str(&env, "DSLR Camera"),
        &ItemCategory::Equipment,
        &String::from_str(&env, "Canon EOS 200D"),
        &String::from_str(&env, "Multimedia Lab"),
        &1u32,
    );

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    client.borrow_item(
        &user1,
        &1u64,
        &String::from_str(&env, "Event documentation"),
        &2u64,
    );

    let result = client.borrow_item(
        &user2,
        &1u64,
        &String::from_str(&env, "Other purpose"),
        &1u64,
    );
    assert!(result.to_string().contains("Error"));
    assert!(result.to_string().contains("unavailable"));
}

#[test]
fn test_borrow_nonexistent_item_fails() {
    let (env, admin, student) = setup();
    let client = init_contract(&env, &admin);

    let result = client.borrow_item(
        &student,
        &999u64,
        &String::from_str(&env, "Test"),
        &1u64,
    );
    assert!(result.to_string().contains("Error"));
}

// ========================
// RETURN ITEM TESTS
// ========================

#[test]
fn test_return_item_on_time() {
    let (env, admin, student) = setup();
    let client = init_contract(&env, &admin);
    add_sample_items(&env, &client, &admin);

    client.borrow_item(
        &student,
        &2u64,
        &String::from_str(&env, "Basketball practice"),
        &3u64,
    );

    let result = client.return_item(&student, &1u64);
    assert!(result.to_string().contains("successfully"));

    let items = client.get_all_items();
    assert_eq!(items.get(1).unwrap().available_quantity, 5);

    let loans = client.get_all_loans();
    assert_eq!(loans.get(0).unwrap().status, LoanStatus::Completed);
}

#[test]
fn test_return_item_overdue() {
    let (env, admin, student) = setup();
    let client = init_contract(&env, &admin);
    add_sample_items(&env, &client, &admin);

    client.borrow_item(
        &student,
        &1u64,
        &String::from_str(&env, "Department meeting"),
        &1u64,
    );

    // Advance time past the deadline (3 days later)
    env.ledger().set(LedgerInfo {
        timestamp: 1_700_000_000 + (3 * 86400),
        ..Default::default()
    });

    let result = client.return_item(&student, &1u64);
    assert!(result.to_string().contains("successfully"));

    let loans = client.get_all_loans();
    assert_eq!(loans.get(0).unwrap().status, LoanStatus::Overdue);
}

#[test]
fn test_return_by_different_user_fails() {
    let (env, admin, student) = setup();
    let client = init_contract(&env, &admin);
    add_sample_items(&env, &client, &admin);

    client.borrow_item(
        &student,
        &1u64,
        &String::from_str(&env, "Test"),
        &2u64,
    );

    let other_user = Address::generate(&env);
    let result = client.return_item(&other_user, &1u64);
    assert!(result.to_string().contains("Error"));
}

// ========================
// EXTEND LOAN TESTS
// ========================

#[test]
fn test_extend_loan_success() {
    let (env, admin, student) = setup();
    let client = init_contract(&env, &admin);
    add_sample_items(&env, &client, &admin);

    client.borrow_item(
        &student,
        &1u64,
        &String::from_str(&env, "National seminar"),
        &3u64,
    );

    let result = client.extend_loan(&student, &1u64, &5u64);
    assert!(result.to_string().contains("successfully"));

    let loans = client.get_all_loans();
    let loan = loans.get(0).unwrap();
    assert!(loan.extended);
    // Planned return = initial + 3 days + 5 days extension = 8 days total
    let expected = 1_700_000_000 + (8 * 86400);
    assert_eq!(loan.planned_return_date, expected);
}

#[test]
fn test_extend_twice_fails() {
    let (env, admin, student) = setup();
    let client = init_contract(&env, &admin);
    add_sample_items(&env, &client, &admin);

    client.borrow_item(
        &student,
        &1u64,
        &String::from_str(&env, "Seminar"),
        &3u64,
    );

    client.extend_loan(&student, &1u64, &3u64);

    // Second extension should fail
    let result = client.extend_loan(&student, &1u64, &2u64);
    assert!(result.to_string().contains("Error"));
    assert!(result.to_string().contains("already been extended"));
}

#[test]
fn test_extend_past_deadline_fails() {
    let (env, admin, student) = setup();
    let client = init_contract(&env, &admin);
    add_sample_items(&env, &client, &admin);

    client.borrow_item(
        &student,
        &1u64,
        &String::from_str(&env, "Test"),
        &1u64,
    );

    // Advance time past the deadline
    env.ledger().set(LedgerInfo {
        timestamp: 1_700_000_000 + (5 * 86400),
        ..Default::default()
    });

    let result = client.extend_loan(&student, &1u64, &3u64);
    assert!(result.to_string().contains("Error"));
    assert!(result.to_string().contains("passed the deadline"));
}

#[test]
fn test_extend_by_different_user_fails() {
    let (env, admin, student) = setup();
    let client = init_contract(&env, &admin);
    add_sample_items(&env, &client, &admin);

    client.borrow_item(
        &student,
        &1u64,
        &String::from_str(&env, "Test"),
        &3u64,
    );

    let other_user = Address::generate(&env);
    let result = client.extend_loan(&other_user, &1u64, &3u64);
    assert!(result.to_string().contains("Error"));
}

// ========================
// QUERY TESTS
// ========================

#[test]
fn test_get_available_items() {
    let (env, admin, student) = setup();
    let client = init_contract(&env, &admin);

    client.add_item(
        &admin,
        &String::from_str(&env, "Presentation Laptop"),
        &ItemCategory::Electronics,
        &String::from_str(&env, "Laptop for presentations"),
        &String::from_str(&env, "Secretary Office"),
        &1u32,
    );
    client.add_item(
        &admin,
        &String::from_str(&env, "Yoga Mat"),
        &ItemCategory::Sports,
        &String::from_str(&env, "10mm thick yoga mat"),
        &String::from_str(&env, "Sports Storage"),
        &3u32,
    );

    client.borrow_item(
        &student,
        &1u64,
        &String::from_str(&env, "Thesis defense"),
        &1u64,
    );

    let available = client.get_available_items();
    assert_eq!(available.len(), 1);
    assert_eq!(available.get(0).unwrap().id, 2);
}

#[test]
fn test_get_items_by_category() {
    let (env, admin, _) = setup();
    let client = init_contract(&env, &admin);
    add_sample_items(&env, &client, &admin);

    // Add another electronics item
    client.add_item(
        &admin,
        &String::from_str(&env, "Wireless Microphone"),
        &ItemCategory::Electronics,
        &String::from_str(&env, "Shure wireless microphone"),
        &String::from_str(&env, "AV Storage"),
        &2u32,
    );

    let electronics = client.get_items_by_category(&ItemCategory::Electronics);
    assert_eq!(electronics.len(), 2);

    let sports = client.get_items_by_category(&ItemCategory::Sports);
    assert_eq!(sports.len(), 1);

    let rooms = client.get_items_by_category(&ItemCategory::Rooms);
    assert_eq!(rooms.len(), 1);

    let equipment = client.get_items_by_category(&ItemCategory::Equipment);
    assert_eq!(equipment.len(), 0);
}

#[test]
fn test_get_loans_by_user() {
    let (env, admin, student) = setup();
    let client = init_contract(&env, &admin);
    add_sample_items(&env, &client, &admin);

    let other_user = Address::generate(&env);

    client.borrow_item(
        &student,
        &1u64,
        &String::from_str(&env, "Purpose 1"),
        &1u64,
    );
    client.borrow_item(
        &student,
        &2u64,
        &String::from_str(&env, "Purpose 2"),
        &2u64,
    );
    client.borrow_item(
        &other_user,
        &3u64,
        &String::from_str(&env, "Other purpose"),
        &1u64,
    );

    let student_loans = client.get_loans_by_user(&student);
    assert_eq!(student_loans.len(), 2);

    let other_loans = client.get_loans_by_user(&other_user);
    assert_eq!(other_loans.len(), 1);
}

#[test]
fn test_get_active_loans() {
    let (env, admin, student) = setup();
    let client = init_contract(&env, &admin);
    add_sample_items(&env, &client, &admin);

    client.borrow_item(
        &student,
        &1u64,
        &String::from_str(&env, "Test 1"),
        &3u64,
    );
    client.borrow_item(
        &student,
        &2u64,
        &String::from_str(&env, "Test 2"),
        &5u64,
    );

    client.return_item(&student, &1u64);

    let active = client.get_active_loans();
    assert_eq!(active.len(), 1);
    assert_eq!(active.get(0).unwrap().id, 2);
}

#[test]
fn test_get_overdue_loans() {
    let (env, admin, student) = setup();
    let client = init_contract(&env, &admin);
    add_sample_items(&env, &client, &admin);

    client.borrow_item(
        &student,
        &1u64,
        &String::from_str(&env, "Test"),
        &1u64,
    );

    let overdue_initial = client.get_overdue_loans();
    assert_eq!(overdue_initial.len(), 0);

    env.ledger().set(LedgerInfo {
        timestamp: 1_700_000_000 + (5 * 86400),
        ..Default::default()
    });

    let overdue = client.get_overdue_loans();
    assert_eq!(overdue.len(), 1);
}

// ========================
// STATISTICS TESTS
// ========================

#[test]
fn test_get_statistics() {
    let (env, admin, student) = setup();
    let client = init_contract(&env, &admin);
    add_sample_items(&env, &client, &admin);

    // Borrow 2 items
    client.borrow_item(
        &student,
        &1u64,
        &String::from_str(&env, "Test 1"),
        &3u64,
    );
    client.borrow_item(
        &student,
        &2u64,
        &String::from_str(&env, "Test 2"),
        &5u64,
    );

    // Return 1 item
    client.return_item(&student, &1u64);

    let stats = client.get_statistics();
    assert_eq!(stats.total_item_types, 3);
    assert_eq!(stats.total_units, 9);       // 3 + 5 + 1
    assert_eq!(stats.available_units, 8);   // 3 + 4 + 1 (1 still borrowed)
    assert_eq!(stats.borrowed_units, 1);
    assert_eq!(stats.total_loans, 2);
    assert_eq!(stats.active_loans, 1);
    assert_eq!(stats.completed_loans, 1);
    assert_eq!(stats.overdue_loans, 0);
}

// ========================
// UPDATE & REMOVE TESTS
// ========================

#[test]
fn test_update_item_status() {
    let (env, admin, _) = setup();
    let client = init_contract(&env, &admin);
    add_sample_items(&env, &client, &admin);

    let result = client.update_item_status(
        &admin,
        &1u64,
        &ItemStatus::UnderMaintenance,
    );
    assert!(result.to_string().contains("successfully"));

    let items = client.get_all_items();
    assert_eq!(items.get(0).unwrap().status, ItemStatus::UnderMaintenance);
}

#[test]
fn test_remove_item_success() {
    let (env, admin, _) = setup();
    let client = init_contract(&env, &admin);
    add_sample_items(&env, &client, &admin);

    let result = client.remove_item(&admin, &2u64);
    assert!(result.to_string().contains("successfully"));

    let items = client.get_all_items();
    assert_eq!(items.len(), 2);
}

#[test]
fn test_remove_borrowed_item_fails() {
    let (env, admin, student) = setup();
    let client = init_contract(&env, &admin);

    client.add_item(
        &admin,
        &String::from_str(&env, "Laptop"),
        &ItemCategory::Electronics,
        &String::from_str(&env, "Laptop"),
        &String::from_str(&env, "Lab"),
        &1u32,
    );
    client.borrow_item(
        &student,
        &1u64,
        &String::from_str(&env, "Thesis defense"),
        &1u64,
    );

    let result = client.remove_item(&admin, &1u64);
    assert!(result.to_string().contains("Error"));
}

#[test]
fn test_borrow_under_maintenance_fails() {
    let (env, admin, student) = setup();
    let client = init_contract(&env, &admin);
    add_sample_items(&env, &client, &admin);

    // Set item to under maintenance
    client.update_item_status(&admin, &1u64, &ItemStatus::UnderMaintenance);

    // Attempt to borrow — should fail
    let result = client.borrow_item(
        &student,
        &1u64,
        &String::from_str(&env, "Test"),
        &1u64,
    );
    assert!(result.to_string().contains("Error"));
    assert!(result.to_string().contains("maintenance"));
}

#[test]
fn test_update_item_quantity() {
    let (env, admin, _) = setup();
    let client = init_contract(&env, &admin);
    add_sample_items(&env, &client, &admin);

    let result = client.update_item_quantity(&admin, &1u64, &2u32);
    assert!(result.to_string().contains("successfully"));

    let items = client.get_all_items();
    assert_eq!(items.get(0).unwrap().total_quantity, 5);
    assert_eq!(items.get(0).unwrap().available_quantity, 5);
}
