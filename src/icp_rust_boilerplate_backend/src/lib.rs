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

#[derive(candid::CandidType, Serialize, Deserialize, Clone, Default, Debug)]
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
            created_at: time(),
        }
    }
}

#[derive(candid::CandidType, Serialize, Deserialize, Clone, Default, Debug)]
struct LeaseAgreement {
    id: u64,
    property_id: u64,
    tenant: String,
    rent: f64,
    start_date: u64,
    end_date: u64,
    created_at: u64,
    digital_signature: String,
}

impl LeaseAgreement {
    fn new(
        id: u64,
        property_id: u64,
        tenant: String,
        rent: f64,
        start_date: u64,
        end_date: u64,
        digital_signature: String,
    ) -> Self {
        Self {
            id,
            property_id,
            tenant,
            rent,
            start_date,
            end_date,
            created_at: time(),
            digital_signature,
        }
    }
}

#[derive(candid::CandidType, Serialize, Deserialize, Clone, Default, Debug)]
struct MaintenanceRequest {
    id: u64,
    property_id: u64,
    description: String,
    status: String,
    created_at: u64,
    priority: String,
}

impl MaintenanceRequest {
    fn new(id: u64, property_id: u64, description: String, status: String, priority: String) -> Self {
        Self {
            id,
            property_id,
            description,
            status,
            created_at: time(),
            priority,
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

#[derive(candid::CandidType, Deserialize, Serialize, Clone)]
struct PropertyPayload {
    address: String,
    owner: String,
    valuation: f64,
    status: String,
}

#[derive(candid::CandidType, Deserialize, Serialize, Clone)]
struct LeaseAgreementPayload {
    property_id: u64,
    tenant: String,
    rent: f64,
    start_date: u64,
    end_date: u64,
    digital_signature: String,
}

#[derive(candid::CandidType, Deserialize, Serialize, Clone)]
struct MaintenanceRequestPayload {
    property_id: u64,
    description: String,
    status: String,
    priority: String,
}

// Smart Contract Functions

#[ic_cdk::update]
fn create_property(payload: PropertyPayload) -> Result<Property, Error> {
    if payload.address.is_empty() || payload.owner.is_empty() {
        return Err(Error::ValidationError {
            msg: "Address and owner are required".to_string(),
        });
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
    ic_cdk::println!("Property created: {:?}", property);
    Ok(property)
}

#[ic_cdk::update]
fn update_property(id: u64, payload: PropertyPayload) -> Result<Property, Error> {
    PROPERTIES_STORAGE.with(|storage| {
        let mut storage = storage.borrow_mut();
        if let Some(mut property) = storage.get(&id) {
            property.address = payload.address;
            property.owner = payload.owner;
            property.valuation = payload.valuation;
            property.status = payload.status;
            storage.insert(id, property.clone());
            ic_cdk::println!("Property updated: {:?}", property);
            Ok(property)
        } else {
            Err(Error::NotFound {
                msg: "Property not found".to_string(),
            })
        }
    })
}

#[ic_cdk::update]
fn delete_property(id: u64) -> Result<(), Error> {
    PROPERTIES_STORAGE.with(|storage| {
        let mut storage = storage.borrow_mut();
        if storage.remove(&id).is_some() {
            ic_cdk::println!("Property deleted: ID {}", id);
            Ok(())
        } else {
            Err(Error::NotFound {
                msg: "Property not found".to_string(),
            })
        }
    })
}

#[ic_cdk::query]
fn get_all_properties() -> Result<Vec<Property>, Error> {
    PROPERTIES_STORAGE.with(|storage| {
        let properties = storage.borrow().iter().map(|(_, property)| property.clone()).collect::<Vec<_>>();
        if properties.is_empty() {
            Err(Error::NotFound {
                msg: "No properties found.".to_string(),
            })
        } else {
            Ok(properties)
        }
    })
}

#[ic_cdk::update]
fn create_lease_agreement(payload: LeaseAgreementPayload) -> Result<LeaseAgreement, Error> {
    if payload.tenant.is_empty() {
        return Err(Error::ValidationError {
            msg: "Tenant name is required".to_string(),
        });
    }
    if !PROPERTIES_STORAGE.with(|storage| storage.borrow().contains_key(&payload.property_id)) {
        return Err(Error::NotFound {
            msg: "Property not found".to_string(),
        });
    }
    if payload.start_date >= payload.end_date {
        return Err(Error::ValidationError {
            msg: "Invalid dates. Start date must be before end date".to_string(),
        });
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
        payload.digital_signature,
    );
    LEASES_STORAGE.with(|storage| storage.borrow_mut().insert(lease.id, lease.clone()));
    ic_cdk::println!("Lease agreement created: {:?}", lease);
    Ok(lease)
}

#[ic_cdk::update]
fn update_lease_agreement(id: u64, payload: LeaseAgreementPayload) -> Result<LeaseAgreement, Error> {
    LEASES_STORAGE.with(|storage| {
        let mut storage = storage.borrow_mut();
        if let Some(mut lease) = storage.get(&id) {
            lease.property_id = payload.property_id;
            lease.tenant = payload.tenant;
            lease.rent = payload.rent;
            lease.start_date = payload.start_date;
            lease.end_date = payload.end_date;
            lease.digital_signature = payload.digital_signature;
            storage.insert(id, lease.clone());
            ic_cdk::println!("Lease agreement updated: {:?}", lease);
            Ok(lease)
        } else {
            Err(Error::NotFound {
                msg: "Lease agreement not found".to_string(),
            })
        }
    })
}

#[ic_cdk::update]
fn delete_lease_agreement(id: u64) -> Result<(), Error> {
    LEASES_STORAGE.with(|storage| {
        let mut storage = storage.borrow_mut();
        if storage.remove(&id).is_some() {
            ic_cdk::println!("Lease agreement deleted: ID {}", id);
            Ok(())
        } else {
            Err(Error::NotFound {
                msg: "Lease agreement not found".to_string(),
            })
        }
    })
}

#[ic_cdk::query]
fn get_all_lease_agreements() -> Result<Vec<LeaseAgreement>, Error> {
    LEASES_STORAGE.with(|storage| {
        let leases = storage
            .borrow()
            .iter()
            .map(|(_, lease)| lease.clone())
            .collect::<Vec<_>>();
        if leases.is_empty() {
            Err(Error::NotFound {
                msg: "No lease agreements found.".to_string(),
            })
        } else {
            Ok(leases)
        }
    })
}

#[ic_cdk::update]
fn create_maintenance_request(
    payload: MaintenanceRequestPayload,
) -> Result<MaintenanceRequest, Error> {
    if payload.status != "pending" && payload.status != "completed" {
        return Err(Error::ValidationError {
            msg: "Invalid status. Status must be either 'pending' or 'completed'".to_string(),
        });
    }
    if !PROPERTIES_STORAGE.with(|storage| storage.borrow().contains_key(&payload.property_id)) {
        return Err(Error::NotFound {
            msg: "Property not found".to_string(),
        });
    }
    let id = ID_COUNTER.with(|counter| {
        let current_value = *counter.borrow().get();
        counter
            .borrow_mut()
            .set(current_value + 1)
            .expect("Failed to increment ID counter");
        current_value
    });
    let request = MaintenanceRequest::new(
        id,
        payload.property_id,
        payload.description,
        payload.status,
        payload.priority,
    );
    MAINTENANCE_STORAGE.with(|storage| storage.borrow_mut().insert(request.id, request.clone()));
    ic_cdk::println!("Maintenance request created: {:?}", request);
    Ok(request)
}

#[ic_cdk::update]
fn update_maintenance_request(id: u64, payload: MaintenanceRequestPayload) -> Result<MaintenanceRequest, Error> {
    MAINTENANCE_STORAGE.with(|storage| {
        let mut storage = storage.borrow_mut();
        if let Some(mut request) = storage.get(&id) {
            request.property_id = payload.property_id;
            request.description = payload.description;
            request.status = payload.status;
            request.priority = payload.priority;
            storage.insert(id, request.clone());
            ic_cdk::println!("Maintenance request updated: {:?}", request);
            Ok(request)
        } else {
            Err(Error::NotFound {
                msg: "Maintenance request not found".to_string(),
            })
        }
    })
}

#[ic_cdk::update]
fn delete_maintenance_request(id: u64) -> Result<(), Error> {
    MAINTENANCE_STORAGE.with(|storage| {
        let mut storage = storage.borrow_mut();
        if storage.remove(&id).is_some() {
            ic_cdk::println!("Maintenance request deleted: ID {}", id);
            Ok(())
        } else {
            Err(Error::NotFound {
                msg: "Maintenance request not found".to_string(),
            })
        }
    })
}

#[ic_cdk::query]
fn get_all_maintenance_requests() -> Result<Vec<MaintenanceRequest>, Error> {
    MAINTENANCE_STORAGE.with(|storage| {
        let requests = storage
            .borrow()
            .iter()
            .map(|(_, request)| request.clone())
            .collect::<Vec<_>>();
        if requests.is_empty() {
            Err(Error::NotFound {
                msg: "No maintenance requests found.".to_string(),
            })
        } else {
            Ok(requests)
        }
    })
}

// Implement Storable and BoundedStorable for Data Structures

impl Storable for Property {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Property {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

impl Storable for LeaseAgreement {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for LeaseAgreement {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

impl Storable for MaintenanceRequest {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for MaintenanceRequest {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

// Error Types

#[derive(candid::CandidType, Deserialize, Serialize, Debug)]
enum Error {
    NotFound { msg: String },
    ValidationError { msg: String },
    Unauthorized { msg: String },
}

impl From<Error> for String {
    fn from(error: Error) -> Self {
        match error {
            Error::NotFound { msg } => msg,
            Error::ValidationError { msg } => msg,
            Error::Unauthorized { msg } => msg,
        }
    }
}

// Generate Candid

ic_cdk::export_candid!();

// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_property() {
        let payload = PropertyPayload {
            address: "123 Main St".to_string(),
            owner: "Alice".to_string(),
            valuation: 500000.0,
            status: "available".to_string(),
        };
        let property = create_property(payload.clone()).unwrap();
        assert_eq!(property.address, "123 Main St");
        assert_eq!(property.owner, "Alice");
    }

    #[test]
    fn test_update_property() {
        let payload = PropertyPayload {
            address: "123 Main St".to_string(),
            owner: "Alice".to_string(),
            valuation: 500000.0,
            status: "available".to_string(),
        };
        let property = create_property(payload.clone()).unwrap();
        let updated_payload = PropertyPayload {
            address: "123 Main St".to_string(),
            owner: "Bob".to_string(),
            valuation: 600000.0,
            status: "sold".to_string(),
        };
        let updated_property = update_property(property.id, updated_payload).unwrap();
        assert_eq!(updated_property.owner, "Bob");
        assert_eq!(updated_property.valuation, 600000.0);
    }

    #[test]
    fn test_delete_property() {
        let payload = PropertyPayload {
            address: "123 Main St".to_string(),
            owner: "Alice".to_string(),
            valuation: 500000.0,
            status: "available".to_string(),
        };
        let property = create_property(payload).unwrap();
        delete_property(property.id).unwrap();
        let result = get_all_properties();
        assert!(result.is_err());
    }
}