#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

// Data Structures

#[derive(candid::CandidType, Serialize, Deserialize, Clone, Default)]
struct Property {
    id: u64,
    address: String,
    owner: String,
    valuation: f64,
    status: String,
    created_at: u64,
}

impl Property {
    fn new(id: u64, address: String, owner: String, valuation: f64, status: String) -> Self {
        Self {
            id,
            address,
            owner,
            valuation,
            status,
            created_at: time() / 1_000_000_000, // Convert nanoseconds to seconds
        }
    }
}

#[derive(candid::CandidType, Serialize, Deserialize, Clone, Default)]
struct LeaseAgreement {
    id: u64,
    property_id: u64,
    tenant: String,
    rent: f64,
    start_date: u64,
    end_date: u64,
    created_at: u64,
    digital_signature: String, // Added field for digital signature
}

impl LeaseAgreement {
    fn new(
        id: u64,
        property_id: u64,
        tenant: String,
        rent: f64,
        start_date: u64,
        end_date: u64,
        digital_signature: String, // Added parameter for digital signature
    ) -> Self {
        Self {
            id,
            property_id,
            tenant,
            rent,
            start_date,
            end_date,
            created_at: time() / 1_000_000_000, // Convert nanoseconds to seconds
            digital_signature, // Initialize digital signature
        }
    }
}

#[derive(candid::CandidType, Serialize, Deserialize, Clone, Default)]
struct MaintenanceRequest {
    id: u64,
    property_id: u64,
    description: String,
    status: String,
    created_at: u64,
    priority: String, // Added field for priority
}

impl MaintenanceRequest {
    fn new(id: u64, property_id: u64, description: String, status: String, priority: String) -> Self {
        Self {
            id,
            property_id,
            description,
            status,
            created_at: time() / 1_000_000_000, // Convert nanoseconds to seconds
            priority, // Initialize priority
        }
    }
}

// Storage Mechanisms

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static PROPERTIES_STORAGE: RefCell<StableBTreeMap<u64, Property, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));

    static LEASES_STORAGE: RefCell<StableBTreeMap<u64, LeaseAgreement, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2)))
    ));

    static MAINTENANCE_STORAGE: RefCell<StableBTreeMap<u64, MaintenanceRequest, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(3)))
    ));
}

// Payload Definitions

#[derive(candid::CandidType, Deserialize, Serialize)]
struct PropertyPayload {
    address: String,
    owner: String,
    valuation: f64,
    status: String,
}

#[derive(candid::CandidType, Deserialize, Serialize)]
struct LeaseAgreementPayload {
    property_id: u64,
    tenant: String,
    rent: f64,
    start_date: u64,
    end_date: u64,
    digital_signature: String, // Added field for digital signature
}

#[derive(candid::CandidType, Deserialize, Serialize)]
struct MaintenanceRequestPayload {
    property_id: u64,
    description: String,
    status: String,
    priority: String, // Added field for priority
}

// Smart Contract Functions

/// Creates a new property and stores it in the stable storage.
/// 
/// # Arguments
/// 
/// * `payload` - A `PropertyPayload` containing the details of the property.
/// 
/// # Returns
/// 
/// * `Result<Property, String>` - The created property or an error message.
#[ic_cdk::update]
fn create_property(payload: PropertyPayload) -> Result<Property, String> {
    // Validate the payload to ensure all fields are provided
    if payload.address.is_empty() || payload.owner.is_empty() {
        return Err("Address and owner are required".to_string());
    }
    let id = ID_COUNTER.with(|counter| {
        let current_value = *counter.borrow().get();
        counter
            .borrow_mut()
            .set(current_value + 1)
            .expect("Failed to increment ID counter");
        current_value
    });
    let property = Property::new(
        id,
        payload.address,
        payload.owner,
        payload.valuation,
        payload.status,
    );
    PROPERTIES_STORAGE.with(|storage| storage.borrow_mut().insert(property.id, property.clone()));
    Ok(property)
}

/// Retrieves all properties from the stable storage.
/// 
/// # Returns
/// 
/// * `Result<Vec<Property>, String>` - A vector of properties or an error message.
#[ic_cdk::query]
fn get_all_properties() -> Result<Vec<Property>, String> {
    PROPERTIES_STORAGE.with(|storage| {
        let properties = storage.borrow().iter().map(|(_, property)| property.clone()).collect::<Vec<_>>();
        if properties.is_empty() {
            Err("No properties found.".to_string())
        } else {
            Ok(properties)
        }
    })
}

/// Validates the lease agreement payload.
/// 
/// # Arguments
/// 
/// * `payload` - A `LeaseAgreementPayload` containing the details of the lease agreement.
/// 
/// # Returns
/// 
/// * `Result<(), String>` - Ok if valid, or an error message.
fn validate_lease_agreement_payload(payload: &LeaseAgreementPayload) -> Result<(), String> {
    if payload.tenant.is_empty() {
        return Err("Tenant name is required".to_string());
    }
    if payload.start_date >= payload.end_date {
        return Err("Invalid dates. Start date must be before end date".to_string());
    }
    Ok(())
}

/// Creates a new lease agreement and stores it in the stable storage.
/// 
/// # Arguments
/// 
/// * `payload` - A `LeaseAgreementPayload` containing the details of the lease agreement.
/// 
/// # Returns
/// 
/// * `Result<LeaseAgreement, String>` - The created lease agreement or an error message.
#[ic_cdk::update]
fn create_lease_agreement(payload: LeaseAgreementPayload) -> Result<LeaseAgreement, String> {
    // Validate the payload to ensure all fields are provided
    validate_lease_agreement_payload(&payload)?;
    // Validate the property ID
    if !PROPERTIES_STORAGE.with(|storage| storage.borrow().contains_key(&payload.property_id)) {
        return Err("Property not found".to_string());
    }
    let id = ID_COUNTER.with(|counter| {
        let current_value = *counter.borrow().get();
        counter
            .borrow_mut()
            .set(current_value + 1)
            .expect("Failed to increment ID counter");
        current_value
    });
    let lease = LeaseAgreement::new(
        id,
        payload.property_id,
        payload.tenant,
        payload.rent,
        payload.start_date,
        payload.end_date,
        payload.digital_signature, // Include digital signature
    );
    LEASES_STORAGE.with(|storage| storage.borrow_mut().insert(lease.id, lease.clone()));
    Ok(lease)
}

/// Retrieves all lease agreements from the stable storage.
/// 
/// # Returns
/// 
/// * `Result<Vec<LeaseAgreement>, String>` - A vector of lease agreements or an error message.
#[ic_cdk::query]
fn get_all_lease_agreements() -> Result<Vec<LeaseAgreement>, String> {
    LEASES_STORAGE.with(|storage| {
        let leases = storage
            .borrow()
            .iter()
            .map(|(_, lease)| lease.clone())
            .collect::<Vec<_>>();
        if leases.is_empty() {
            Err("No lease agreements found.".to_string())
        } else {
            Ok(leases)
        }
    })
}

/// Validates the maintenance request payload.
/// 
/// # Arguments
/// 
/// * `payload` - A `MaintenanceRequestPayload` containing the details of the maintenance request.
/// 
/// # Returns
/// 
/// * `Result<(), String>` - Ok if valid, or an error message.
fn validate_maintenance_request_payload(payload: &MaintenanceRequestPayload) -> Result<(), String> {
    if payload.status != "pending" && payload.status != "completed" {
        return Err("Invalid status. Status must be either 'pending' or 'completed'".to_string());
    }
    Ok(())
}

/// Creates a new maintenance request and stores it in the stable storage.
/// 
/// # Arguments
/// 
/// * `payload` - A `MaintenanceRequestPayload` containing the details of the maintenance request.
/// 
/// # Returns
/// 
/// * `Result<MaintenanceRequest, String>` - The created maintenance request or an error message.
#[ic_cdk::update]
fn create_maintenance_request(
    payload: MaintenanceRequestPayload,
) -> Result<MaintenanceRequest, String> {
    // Validate the user input
    validate_maintenance_request_payload(&payload)?;
    // Validate the property ID
    if !PROPERTIES_STORAGE.with(|storage| storage.borrow().contains_key(&payload.property_id)) {
        return Err("Property not found".to_string());
    }
    // Create the maintenance request
    let id = ID_COUNTER.with(|counter| {
        let current_value = *counter.borrow().get();
        counter
            .borrow_mut()
            .set(current_value + 1)
            .expect("Failed to increment ID counter");
        current_value
    });
    let request =
        MaintenanceRequest::new(id, payload.property_id, payload.description, payload.status, payload.priority); // Include priority
    MAINTENANCE_STORAGE.with(|storage| storage.borrow_mut().insert(request.id, request.clone()));
    Ok(request)
}

/// Retrieves all maintenance requests from the stable storage.
/// 
/// # Returns
/// 
/// * `Result<Vec<MaintenanceRequest>, String>` - A vector of maintenance requests or an error message.
#[ic_cdk::query]
fn get_all_maintenance_requests() -> Result<Vec<MaintenanceRequest>, String> {
    MAINTENANCE_STORAGE.with(|storage| {
        let requests = storage
            .borrow()
            .iter()
            .map(|(_, request)| request.clone())
            .collect::<Vec<_>>();
        if requests.is_empty() {
            Err("No maintenance requests found.".to_string())
        } else {
            Ok(requests)
        }
    })
}

// Implement Storable and BoundedStorable for Data Structures

impl Storable for Property {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Encoding failed"))
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Decoding failed")
    }
}

impl BoundedStorable for Property {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

impl Storable for LeaseAgreement {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Encoding failed"))
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Decoding failed")
    }
}

impl BoundedStorable for LeaseAgreement {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

impl Storable for MaintenanceRequest {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Encoding failed"))
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Decoding failed")
    }
}

impl BoundedStorable for MaintenanceRequest {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

// Error Types

#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    NotFound { msg: String },
    UnAuthorized { msg: String },
}

// Generate Candid

ic_cdk::export_candid!();