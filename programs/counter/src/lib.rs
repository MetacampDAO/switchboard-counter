use anchor_lang::*;
pub use switchboard_solana::prelude::anchor_lang;
pub use switchboard_solana::prelude::anchor_spl;
pub use switchboard_solana::prelude::*;
pub mod error;
pub use error::*;

declare_id!("7vdpJaVD83HZPSroATuxfkYfrg72yfxe35cC4xMkFUNM");

pub const PROGRAM_SEED: &[u8] = b"CUSTOM_RANDOMNESS";
pub const GUESS_COST: u64 = 100_000;
pub const GLOBAL_VAR: &[u8] = b"global6";

#[program]
pub mod counter {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.global.count = 0;

        let mut user = ctx.accounts.user.load_init()?;
        user.bump = *ctx.bumps.get("user").unwrap();
        user.authority = ctx.accounts.initializer.key();
        user.token_wallet = ctx.accounts.user_token_wallet.key();
        Ok(())
    }

    pub fn add_one(ctx: Context<AddOne>) -> Result<()> {
        if ctx.accounts.user_token_wallet.amount < GUESS_COST {
            switchboard_solana::wrap_native(
                &ctx.accounts.system_program,
                &ctx.accounts.token_program,
                &ctx.accounts.user_token_wallet,
                &ctx.accounts.initializer,
                &[&[
                    PROGRAM_SEED,
                    ctx.accounts.user.load()?.authority.key().as_ref(),
                    &[ctx.accounts.user.load()?.bump],
                ]],
                GUESS_COST
                    .checked_sub(ctx.accounts.user_token_wallet.amount)
                    .unwrap(),
            )?;
        }

        ctx.accounts.user_token_wallet.reload()?;

        assert!(
            ctx.accounts.user_token_wallet.amount >= GUESS_COST,
            "User escrow is missing funds"
        );

        let request_params = format!(
            "PID={},MAX_GUESS={},USER={}",
            crate::id(),
            255,
            ctx.accounts.user.key()
        );

        let request_init_ctx = FunctionRequestInitAndTrigger {
            request: ctx.accounts.request.clone(),
            function: ctx.accounts.function.clone(),
            escrow: ctx.accounts.request_escrow.clone(),
            mint: ctx.accounts.mint.clone(),
            state: ctx.accounts.state.clone(),
            attestation_queue: ctx.accounts.attestation_queue.clone(),
            payer: ctx.accounts.initializer.clone(),
            system_program: ctx.accounts.system_program.clone(),
            token_program: ctx.accounts.token_program.clone(),
            associated_token_program: ctx.accounts.associated_token_program.clone(),
        };
        request_init_ctx.invoke(
            ctx.accounts.switchboard.clone(),
            None,
            Some(1000),
            Some(512),
            Some(request_params.into_bytes()),
            None,
        )?;

        ctx.accounts.global.count += 1;
        Ok(())
    }

    pub fn user_settle(ctx: Context<UserSettle>, result: u8) -> Result<()> {
        // verify we havent responded already
        if ctx.accounts.user.load()?.current_round.status != RoundStatus::Pending {
            return Err(error!(RandomnessRequestError::RoundInactive));
        }

        if ctx.accounts.request.active_request.status != RequestStatus::RequestSuccess {
            return Err(error!(
                RandomnessRequestError::SwitchboardRequestNotSuccessful
            ));
        }

        let mut user = ctx.accounts.user.load_mut()?;
        user.current_round.result = result;
        user.current_round.status = RoundStatus::Settled;

        ctx.accounts.global.count += result as u64;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, seeds=[GLOBAL_VAR], bump, payer=initializer, space=8+8)]
    pub global: Account<'info, Global>,
    #[account(
        init,
        space = 8 + std::mem::size_of::<UserState>(),
        payer = initializer,
        seeds = [PROGRAM_SEED, initializer.key().as_ref()],
        bump
    )]
    pub user: AccountLoader<'info, UserState>,

    #[account(
        init,
        payer = initializer,
        associated_token::mint = mint,
        associated_token::authority = user,
      )]
    pub user_token_wallet: Box<Account<'info, TokenAccount>>,
    // TOKEN ACCOUNTS
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    #[account(address = anchor_spl::token::spl_token::native_mint::ID)]
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub initializer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AddOne<'info> {
    #[account(mut, seeds=[GLOBAL_VAR], bump)]
    pub global: Account<'info, Global>,

    #[account(
      mut,
      seeds = [PROGRAM_SEED, initializer.key().as_ref()], // user should be paying for this each time
      bump = user.load()?.bump,
      constraint = user.load()?.authority == initializer.key() && user.load()?.token_wallet == user_token_wallet.key(),
  )]
    pub user: AccountLoader<'info, UserState>,

    // SWITCHBOARD ACCOUNTS
    /// CHECK:
    #[account(executable, address = SWITCHBOARD_ATTESTATION_PROGRAM_ID)]
    pub switchboard: AccountInfo<'info>,
    #[account(
    seeds = [STATE_SEED],
    seeds::program = switchboard.key(),
    bump = state.load()?.bump,
  )]
    pub state: AccountLoader<'info, AttestationProgramState>,
    pub attestation_queue: AccountLoader<'info, AttestationQueueAccountData>,
    #[account(
    mut,
    has_one = attestation_queue,
  )]
    pub function: AccountLoader<'info, FunctionAccountData>,
    /// CHECK:
    #[account(
    mut,
    signer,
    owner = system_program.key(),
    constraint = request.data_len() == 0 && request.lamports() == 0
  )]
    pub request: AccountInfo<'info>,
    /// CHECK:
    #[account(
    mut,
    owner = system_program.key(),
    constraint = request.data_len() == 0 && request.lamports() == 0
  )]
    pub request_escrow: AccountInfo<'info>,

    // TOKEN ACCOUNTS
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    #[account(address = anchor_spl::token::spl_token::native_mint::ID)]
    pub mint: Account<'info, Mint>,
    #[account(
        mut,
        constraint = user_token_wallet.owner == user.key()
    )] // we might wrap funds to this wallet
    pub user_token_wallet: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub initializer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UserSettle<'info> {
    #[account(mut, seeds=[GLOBAL_VAR], bump)]
    pub global: Account<'info, Global>,

    #[account(
        mut,
        seeds = [PROGRAM_SEED, user.load()?.authority.as_ref()],
        bump = user.load()?.bump,
        constraint = user.load()?.token_wallet == user_token_wallet.key(),
    )]
    pub user: AccountLoader<'info, UserState>,

    // SWITCHBOARD ACCOUNTS
    pub function: AccountLoader<'info, FunctionAccountData>,
    #[account(
      constraint = request.validate_signer(
          &function.to_account_info(),
          &enclave_signer.to_account_info()
        )? @ RandomnessRequestError::FunctionValidationFailed,
    )]
    pub request: Box<Account<'info, FunctionRequestAccountData>>,
    pub enclave_signer: Signer<'info>,

    // TOKEN ACCOUNTS
    pub token_program: Program<'info, Token>,
    #[account(address = anchor_spl::token::spl_token::native_mint::ID)]
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub house_token_wallet: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub user_token_wallet: Box<Account<'info, TokenAccount>>,
}

#[account]
pub struct Global {
    count: u64,
}

#[zero_copy(unsafe)]
pub struct UserRound {
    pub request: Pubkey,
    pub guess: u8,
    pub status: RoundStatus,
    pub result: u8,
    pub wager: u64,
    pub slot: u64,
    pub timestamp: i64,
}

#[account(zero_copy(unsafe))]
pub struct UserState {
    pub bump: u8,
    pub authority: Pubkey,
    pub token_wallet: Pubkey,
    pub current_round: UserRound,
    pub last_round: UserRound,
}

#[repr(u8)]
#[derive(Copy, Clone, Default, Debug, Eq, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub enum RoundStatus {
    #[default]
    None = 0,
    Pending,
    Settled,
}
impl From<RoundStatus> for u8 {
    fn from(value: RoundStatus) -> Self {
        match value {
            RoundStatus::Pending => 1,
            RoundStatus::Settled => 2,
            _ => 0,
        }
    }
}
impl From<u8> for RoundStatus {
    fn from(value: u8) -> Self {
        match value {
            1 => RoundStatus::Pending,
            2 => RoundStatus::Settled,
            _ => RoundStatus::default(),
        }
    }
}
