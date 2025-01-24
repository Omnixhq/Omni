use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{AccountInfo, next_account_info},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_program,
    program::invoke,
    system_instruction,
};
use std::collections::{HashMap};

// Proposal State
#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, Default)]
pub struct Proposal {
    pub id: u32,
    pub proposer: Pubkey,
    pub title: String,
    pub description: String,
    pub start_time: u64,
    pub end_time: u64,
    pub voting_options: Vec<String>,  // Example: ["Yes", "No", "Abstain"]
    pub votes: HashMap<Pubkey, u8>, // Voter Pubkey => Vote Index (0,1,2 from voting options)
    pub executed: bool,
     pub target_account: Option<Pubkey>, // Account for a system transfer
      pub transfer_lamports: Option<u64>,
}

// Voting Power Data
#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, Default)]
pub struct VotingPower {
  pub voter: Pubkey,
  pub voting_power: u64,
  pub delegated_to: Option<Pubkey>
}

// Agent Configuration for DAO
#[derive(BorshDeserialize, BorshSerialize, Debug, Clone)]
pub struct AgentConfig {
    pub owner: Pubkey,
    pub description: String,
     pub voting_threshold: f64,  // percentage required for the proposal to pass, eg: 0.6
     pub quorum_threshold: f64, // percentage required to start a proposal
    // Add more DAO specific configs
}

// Agent Instance Structure
#[derive(BorshDeserialize, BorshSerialize, Debug, Clone)]
pub struct AgentInstance {
    pub agent_id: u32,
    pub status: u8,         // 0: created, 1: running, 2: completed, 3: error
    pub start_time: u64,
}

// Program State (Account Data)
#[derive(BorshDeserialize, BorshSerialize, Debug, Default)]
pub struct ProgramState {
    pub next_agent_id: u32,
     pub next_proposal_id: u32,
    pub agent_configs: Vec<AgentConfig>,
    pub agent_instances: Vec<AgentInstance>,
     pub proposals: Vec<Proposal>,
      pub voting_power: HashMap<Pubkey, VotingPower>,
      pub last_analysis_time: u64,
}

// Define Instruction Enum
#[derive(BorshDeserialize, BorshSerialize, Debug, Clone)]
pub enum AgentInstruction {
    CreateAgent(AgentConfig),
    CreateAgentInstance { agent_id: u32 },
    UpdateAgentInstanceStatus { agent_id: u32, instance_id: u32, status: u8 },
     CreateProposal(Proposal),
     VoteOnProposal { proposal_id: u32, vote_index: u8},
     ExecuteProposal { proposal_id: u32},
     DelegateVotingPower { delegate_to: Pubkey },
     UpdateVotingPower { voter: Pubkey, voting_power: u64 },
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
        AgentInstruction::CreateProposal(proposal) => {
           msg!("Creating new proposal...");
           create_proposal(&mut program_state, proposal, state_account)?;
        }
        AgentInstruction::VoteOnProposal{proposal_id, vote_index} => {
            msg!("Voting on proposal...");
           vote_on_proposal(&mut program_state, proposal_id, vote_index, state_account)?;
        }
       AgentInstruction::ExecuteProposal{proposal_id} => {
            msg!("Executing proposal...");
            execute_proposal(&mut program_state, proposal_id, state_account, program_id)?;
        }
       AgentInstruction::DelegateVotingPower{delegate_to} => {
            msg!("Delegating voting power");
             delegate_voting_power(&mut program_state, delegate_to, state_account)?;
        }
       AgentInstruction::UpdateVotingPower{voter, voting_power} => {
            msg!("Updating voting power");
            update_voting_power(&mut program_state, voter, voting_power, state_account)?;
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

fn create_proposal(
    program_state: &mut ProgramState,
    proposal: Proposal,
    _state_account: &AccountInfo,
) -> ProgramResult {
     let mut proposal = proposal.clone();
     proposal.id = program_state.next_proposal_id;
     program_state.proposals.push(proposal);
      program_state.next_proposal_id += 1;

    msg!("Created proposal with ID: {}", proposal.id);
    Ok(())
}

fn vote_on_proposal(
    program_state: &mut ProgramState,
    proposal_id: u32,
    vote_index: u8,
    state_account: &AccountInfo,
) -> ProgramResult {
      if program_state.proposals.len() <= proposal_id as usize {
        msg!("Proposal not found");
         return Err(ProgramError::InvalidArgument);
      }

     let proposal = program_state.proposals.get_mut(proposal_id as usize).unwrap();

       // Check if the voting time frame is open
      let current_time = solana_program::sysvar::clock::Clock::get().unwrap().unix_timestamp as u64;
        if current_time < proposal.start_time || current_time > proposal.end_time {
            msg!("Voting is not open for this proposal.");
            return Err(ProgramError::InvalidArgument);
         }

     let voter = state_account.key;

      // Get the voter voting power
      let mut voter_voting_power = 1;
      let voting_power = program_state.voting_power.get(voter);
      if let Some(voter_details) = voting_power{
            // Get the voting power of the delegated to user if it exists
            let delegate_to = voter_details.delegated_to;
            if let Some(delegate) = delegate_to{
               let delegate_voting_power = program_state.voting_power.get(&delegate);
               if let Some(delegate_details) = delegate_voting_power {
                    voter_voting_power = delegate_details.voting_power;
                }else{
                    voter_voting_power = voter_details.voting_power;
                }
           }else{
                 voter_voting_power = voter_details.voting_power;
           }
      }
     
     // Process the vote only if the user has voting power
     if voter_voting_power > 0 {
         proposal.votes.insert(*voter, vote_index);
     }
    msg!("Vote recorded for proposal with ID: {}", proposal_id);
    Ok(())
}


fn execute_proposal(
    program_state: &mut ProgramState,
    proposal_id: u32,
    _state_account: &AccountInfo,
    program_id: &Pubkey,
) -> ProgramResult {
    if program_state.proposals.len() <= proposal_id as usize {
        msg!("Proposal not found");
         return Err(ProgramError::InvalidArgument);
      }

      let proposal = program_state.proposals.get_mut(proposal_id as usize).unwrap();
      if proposal.executed {
          msg!("Proposal has already been executed.");
          return Err(ProgramError::InvalidArgument);
      }

       // Check if the voting time frame has elapsed
      let current_time = solana_program::sysvar::clock::Clock::get().unwrap().unix_timestamp as u64;
        if current_time < proposal.end_time  {
            msg!("Voting is still open for this proposal.");
             return Err(ProgramError::InvalidArgument);
         }

     // Check Quorum and Thresholds
     let (passed, quorum_met) = check_proposal_result(proposal, program_state);

       if !quorum_met {
            msg!("Proposal failed: Quorum not met");
           return Err(ProgramError::InvalidArgument)
       }

       if !passed {
           msg!("Proposal failed: Vote threshold not met");
           return Err(ProgramError::InvalidArgument)
        }

    // Execute Proposal Logic - system transfer as an example
      if proposal.target_account.is_some() && proposal.transfer_lamports.is_some() {
          msg!("Executing proposal: Transferring lamports.");
             let target_account = proposal.target_account.unwrap();
            let transfer_lamports = proposal.transfer_lamports.unwrap();
            invoke(
                &system_instruction::transfer(
                    &program_id,
                    &target_account,
                     transfer_lamports,
                  ),
                  &[]
             )?;
       }
      proposal.executed = true;
      msg!("Proposal Executed with ID: {}", proposal_id);
      Ok(())
}

fn delegate_voting_power(
    program_state: &mut ProgramState,
    delegate_to: Pubkey,
      state_account: &AccountInfo,
) -> ProgramResult {

    let voter = state_account.key;
    // Fetch the voter details and then update the voting power.
    let voting_power = program_state.voting_power.get_mut(voter);
    if let Some(voting_details) = voting_power{
         voting_details.delegated_to = Some(delegate_to);
    }else{
        let new_voting_details = VotingPower{
            voter: *voter,
            voting_power: 1,
            delegated_to: Some(delegate_to)
        };
        program_state.voting_power.insert(*voter, new_voting_details);
    }
      msg!("Voting power delegated from {:?} to {:?}", voter, delegate_to);
        Ok(())
}

fn update_voting_power(
    program_state: &mut ProgramState,
    voter: Pubkey,
    voting_power: u64,
     _state_account: &AccountInfo,
) -> ProgramResult {

      let voting_details = program_state.voting_power.get_mut(&voter);

        if let Some(voting_power_details) = voting_details {
              voting_power_details.voting_power = voting_power;
        }else{
             let new_voting_details = VotingPower{
                voter: voter,
                voting_power: voting_power,
                delegated_to: None
            };
             program_state.voting_power.insert(voter, new_voting_details);
        }
     msg!("Updated voting power of {:?} to {}", voter, voting_power);
    Ok(())
}

fn check_proposal_result(proposal: &Proposal, program_state: &ProgramState) -> (bool, bool) {
     // Get the total voting power available
     let total_voting_power : u64 = program_state.voting_power.values().fold(0, |acc, x| acc + x.voting_power);

    // Calculate Total number of votes
      let total_voters = proposal.votes.len() as u64;
      let quorum_met =  total_voters as f64 / total_voting_power as f64 >= 0.01;

      if !quorum_met{
        return (false, false);
      }
     
      // Calculate the number of yes votes
      let total_yes_votes = proposal.votes.values().filter(|&vote| *vote == 0).count();

      let vote_threshold_met = total_yes_votes as f64 / total_voters as f64 >= 0.6;
      
      return (vote_threshold_met, quorum_met);

}