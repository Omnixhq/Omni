use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{AccountInfo, next_account_info},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_program,
};
use std::collections::{HashMap};

// Property Structure
#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, Default)]
pub struct Property {
    pub id: u32,
    pub owner: Pubkey,
    pub address: String,
    pub size_sqft: u32,
     pub features: Vec<String>,
    // Add other property details
}

// Transaction Data (Sale or Rental)
#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, Default)]
pub struct Transaction {
    pub property_id: u32,
    pub transaction_type: String,   // "Sale" or "Rental"
    pub price: u64,             // price in lamports
    pub timestamp: u64,          // Time of transaction
    pub buyer: Option<Pubkey>,     // Buyer (for sales)
    pub seller: Option<Pubkey>,   // Seller (for sales)
    pub tenant: Option<Pubkey>,    // Tenant (for rentals)
}

// Market Data (Example - Area Level)
#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, Default)]
pub struct MarketData {
  pub area_name: String,
  pub average_price_sqft: f64,
  pub average_rent_sqft: f64,
}

// Opportunity Struct
#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, Default)]
pub struct Opportunity {
  pub property_id: u32,
  pub opportunity_type: String,
  pub timestamp: u64,
  pub additional_info: String,
}

// Agent Configuration (Real Estate Specific)
#[derive(BorshDeserialize, BorshSerialize, Debug, Clone)]
pub struct AgentConfig {
    pub owner: Pubkey,
    pub description: String,
     pub target_area: String,
    pub desired_cap_rate: f64,
     pub min_roi: f64,
    // Add more real estate-specific settings
}

// Agent Instance Structure
#[derive(BorshDeserialize, BorshSerialize, Debug, Clone)]
pub struct AgentInstance {
    pub agent_id: u32,
    pub status: u8,         // 0: created, 1: running, 2: completed, 3: error
    pub start_time: u64,
    pub triggered_opportunity: Option<Opportunity>,
}

// Program State
#[derive(BorshDeserialize, BorshSerialize, Debug, Default)]
pub struct ProgramState {
    pub next_agent_id: u32,
    pub next_property_id: u32,
    pub agent_configs: Vec<AgentConfig>,
    pub agent_instances: Vec<AgentInstance>,
    pub properties: HashMap<u32, Property>,
    pub transactions: HashMap<u32, Vec<Transaction>>,   // Map property_id to transactions
     pub market_data: HashMap<String, MarketData>,
      pub opportunities: Vec<Opportunity>,
      pub last_analysis_time: u64,
}

// Define Instruction Enum
#[derive(BorshDeserialize, BorshSerialize, Debug, Clone)]
pub enum AgentInstruction {
    CreateAgent(AgentConfig),
    CreateAgentInstance { agent_id: u32 },
    UpdateAgentInstanceStatus { agent_id: u32, instance_id: u32, status: u8 },
     RegisterProperty (Property),
    RecordTransaction {property_id: u32, transaction: Transaction},
      UpdateMarketData { market_data: MarketData},
    AnalyzeRealEstateOpportunities {agent_id: u32},
}

// Entrypoint
entrypoint!(process_instruction);
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("AI Agent Program invoked!");

    let instruction = AgentInstruction::try_from_slice(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

     let accounts_iter = &mut accounts.iter();
    let state_account = next_account_info(accounts_iter)?;

    if !state_account.is_writable {
        msg!("Program state account is not writeable");
        return Err(ProgramError::InvalidArgument);
    }
    
    // Load Program state (if available) or create a new one if not initialized
    let mut program_state = ProgramState::try_from_slice(&state_account.data.borrow())
         .unwrap_or_default();


    match instruction {
        AgentInstruction::CreateAgent(config) => {
            msg!("Creating agent config...");
            create_agent(&mut program_state, config, program_id, state_account)?;
        }
        AgentInstruction::CreateAgentInstance { agent_id } => {
            msg!("Creating agent instance...");
           create_agent_instance(&mut program_state, agent_id, state_account)?;
        }
        AgentInstruction::UpdateAgentInstanceStatus {agent_id, instance_id, status} => {
            msg!("Updating agent instance status...");
             update_agent_instance_status(&mut program_state, agent_id, instance_id, status, state_account)?;
        }
        AgentInstruction::RegisterProperty (property) => {
            msg!("Registering new property...");
            register_property(&mut program_state, property, state_account)?;
        }
        AgentInstruction::RecordTransaction{property_id, transaction} => {
            msg!("Recording Transaction...");
           record_transaction(&mut program_state, property_id, transaction, state_account)?;
        }
        AgentInstruction::UpdateMarketData{market_data} => {
             msg!("Updating market data...");
             update_market_data(&mut program_state, market_data, state_account)?;
        }
       AgentInstruction::AnalyzeRealEstateOpportunities { agent_id } => {
            msg!("Analyzing Real Estate opportunities...");
            analyze_real_estate_opportunities(&mut program_state, agent_id, state_account)?;
       }
    }

     // Serialize the program state back to the account
     program_state.serialize(&mut &mut state_account.data.borrow_mut()[..])?;

    Ok(())
}

// Instruction implementations
fn create_agent(
    program_state: &mut ProgramState,
    config: AgentConfig,
    program_id: &Pubkey,
     state_account: &AccountInfo,
) -> ProgramResult {
    // Check if the signer is the owner of program
     if state_account.owner != program_id {
        msg!("Incorrect owner for program");
        return Err(ProgramError::IncorrectProgramId);
    }
    
    let config_id = program_state.next_agent_id;
    program_state.agent_configs.push(config.clone());
    program_state.next_agent_id += 1;

     msg!("Created agent with ID: {}", config_id);

    Ok(())
}

fn create_agent_instance(
    program_state: &mut ProgramState,
    agent_id: u32,
   _state_account: &AccountInfo,
) -> ProgramResult {
    // Check if agent exists
     if program_state.agent_configs.len() <= agent_id as usize {
        msg!("Agent not found");
        return Err(ProgramError::InvalidArgument);
    }

    let new_instance = AgentInstance {
        agent_id,
        status: 0, // Created status
        start_time: solana_program::sysvar::clock::Clock::get().unwrap().unix_timestamp as u64,
        triggered_opportunity: None,
    };

     program_state.agent_instances.push(new_instance);
     msg!("Created agent instance with agent ID: {}", agent_id);
    Ok(())
}

fn update_agent_instance_status(
    program_state: &mut ProgramState,
    agent_id: u32,
    instance_id: u32,
    status: u8,
    _state_account: &AccountInfo,
) -> ProgramResult {
    if program_state.agent_instances.len() <= instance_id as usize {
        msg!("Agent instance not found");
        return Err(ProgramError::InvalidArgument);
    }

     let instance = program_state.agent_instances.get_mut(instance_id as usize).unwrap();
     if instance.agent_id != agent_id {
        msg!("Incorrect agent ID for the requested instance");
        return Err(ProgramError::InvalidArgument)
    }

     instance.status = status;
      msg!("Updated agent instance status to: {}", status);
     Ok(())
}


fn register_property(
    program_state: &mut ProgramState,
    mut property: Property,
     _state_account: &AccountInfo,
) -> ProgramResult {
    property.id = program_state.next_property_id;
    program_state.properties.insert(property.id, property.clone());
     program_state.next_property_id += 1;

      msg!("Registered Property with ID: {}", property.id);
    Ok(())
}

fn record_transaction(
    program_state: &mut ProgramState,
    property_id: u32,
    transaction: Transaction,
    _state_account: &AccountInfo,
) -> ProgramResult {
       // Check if property exists
       if !program_state.properties.contains_key(&property_id) {
          msg!("Property not found");
          return Err(ProgramError::InvalidArgument);
      }

     let transactions = program_state.transactions.entry(property_id).or_insert_with(Vec::new);
     transactions.push(transaction);

      msg!("Recorded transaction for property with ID: {}", property_id);
    Ok(())
}

fn update_market_data(
     program_state: &mut ProgramState,
      market_data: MarketData,
     _state_account: &AccountInfo,
)->ProgramResult{

      program_state.market_data.insert(market_data.area_name.clone(), market_data);
        Ok(())
}

fn analyze_real_estate_opportunities(
    program_state: &mut ProgramState,
    agent_id: u32,
    _state_account: &AccountInfo,
) -> ProgramResult {

    // Check if agent exists
    if program_state.agent_configs.len() <= agent_id as usize {
        msg!("Agent not found");
        return Err(ProgramError::InvalidArgument);
    }

     let config = &program_state.agent_configs[agent_id as usize];

    // Add the logic for identifying opportunities based on config
      let opportunities = identify_real_estate_opportunities(config, &program_state.properties, &program_state.transactions, &program_state.market_data);

       for opportunity in opportunities {
           program_state.opportunities.push(opportunity.clone());
            // Iterate through instances and trigger if applicable
            for instance in program_state.agent_instances.iter_mut() {
                if instance.agent_id == agent_id && instance.status == 0 {
                     msg!("Triggering instance {}", instance.agent_id);
                    instance.status = 1;
                    instance.triggered_opportunity = Some(opportunity.clone());
                }
           }
      }
      program_state.last_analysis_time =  solana_program::sysvar::clock::Clock::get().unwrap().unix_timestamp as u64;
    Ok(())
}

fn identify_real_estate_opportunities(
    config: &AgentConfig,
    properties: &HashMap<u32, Property>,
    transactions: &HashMap<u32, Vec<Transaction>>,
    market_data: &HashMap<String, MarketData>
) -> Vec<Opportunity> {
     let mut opportunities = Vec::new();

       // Check if Market data exists for the area
    let market_data_for_area = market_data.get(&config.target_area);
    if market_data_for_area.is_none() {
        return opportunities; // No market data available for the area.
    }
     let market_data_area = market_data_for_area.unwrap();

    // Iterate through all properties to perform analysis
      for(property_id, property) in properties{
             //Filter the properties based on the desired area.
          if  !property.address.contains(&config.target_area) {
                 continue;
          }

        let opportunity = check_opportunity_condition(property_id, property, transactions, config, &market_data_area);
         if let Some(opportunity) = opportunity {
              opportunities.push(opportunity);
        }
    }

    opportunities
}


fn check_opportunity_condition(property_id: &u32, property: &Property, transactions: &HashMap<u32, Vec<Transaction>>, config: &AgentConfig, market_data: &MarketData) -> Option<Opportunity>{
         
          let transaction_history = transactions.get(property_id);

          if transaction_history.is_none(){
             return None;
          }

         let transaction_history_properties = transaction_history.unwrap();
        //Get latest sale or rental transaction
          let latest_transaction = transaction_history_properties.iter().max_by_key(|tx| tx.timestamp);
        // Calculate the cap rate (example calculation using latest sale or rent)
        if let Some(latest_transaction) = latest_transaction {
             if latest_transaction.transaction_type == "Rental" {
                let cap_rate = calculate_cap_rate(market_data.average_price_sqft, market_data.average_rent_sqft);
                   if cap_rate >= config.desired_cap_rate {
                        return  Some(Opportunity {
                           property_id: *property_id,
                           opportunity_type: "High Cap Rate".to_string(),
                           timestamp: latest_transaction.timestamp,
                            additional_info: format!("Cap Rate: {:.2}%", cap_rate * 100.0),
                         });
                     }
              }
             
               if latest_transaction.transaction_type == "Sale" {
                   let roi = calculate_roi(latest_transaction.price as f64, market_data.average_price_sqft * property.size_sqft as f64);
                      if roi >= config.min_roi {
                        return Some(Opportunity{
                           property_id: *property_id,
                           opportunity_type: "High ROI".to_string(),
                            timestamp: latest_transaction.timestamp,
                           additional_info: format!("ROI: {:.2}%", roi * 100.0),
                         })
                       }
              }
        }
      None
}

// Example cap rate calculation
fn calculate_cap_rate(average_price_sqft: f64, average_rent_sqft: f64) -> f64 {
    if average_price_sqft == 0.0 {
         return 0.0
    }
    average_rent_sqft / average_price_sqft
}

fn calculate_roi(latest_sale_price: f64, purchase_price: f64 ) -> f64 {
    if purchase_price == 0.0 {
         return 0.0;
    }
     (latest_sale_price - purchase_price) / purchase_price
}