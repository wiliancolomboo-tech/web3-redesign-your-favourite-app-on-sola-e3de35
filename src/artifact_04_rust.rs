use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

declare_id!("SPatrn1111111111111111111111111111111111111");

pub const PROTOCOL_FEE_BPS: u64 = 50; // 0.5% Protocol Fee
pub const BPS_DENOMINATOR: u64 = 10_000;
pub const SECONDS_PER_DAY: i64 = 86_400;

#[program]
pub mod solpatron {
    use super::*;

    /// Initializes the global protocol configuration and platform fee recipient.
    pub fn initialize_protocol(
        ctx: Context<InitializeProtocol>,
        fee_recipient: Pubkey,
    ) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.admin = ctx.accounts.admin.key();
        config.fee_recipient = fee_recipient;
        config.fee_bps = PROTOCOL_FEE_BPS;
        config.total_creators = 0;
        config.bump = ctx.bumps.config;
        Ok(())
    }

    /// Registers a new creator profile and settlement parameters.
    pub fn initialize_creator(
        ctx: Context<InitializeCreator>,
        creator_metadata_uri: String,
    ) -> Result<()> {
        require!(
            creator_metadata_uri.len() <= 200,
            SolPatronError::MetadataUriTooLong
        );
        let creator_profile = &mut ctx.accounts.creator_profile;
        creator_profile.creator_authority = ctx.accounts.creator.key();
        creator_profile.payment_mint = ctx.accounts.payment_mint.key();
        creator_profile.metadata_uri = creator_metadata_uri;
        creator_profile.tier_count = 0;
        creator_profile.active_subscribers = 0;
        creator_profile.total_revenue = 0;
        creator_profile.bump = ctx.bumps.creator_profile;

        let config = &mut ctx.accounts.config;
        config.total_creators = config.total_creators.checked_add(1).unwrap();
        Ok(())
    }

    /// Creates a subscription tier with rate-per-epoch and interval duration.
    pub fn create_tier(
        ctx: Context<CreateTier>,
        tier_id: u8,
        cost_per_period: u64,
        period_duration_seconds: i64,
        tier_name: String,
    ) -> Result<()> {
        require!(cost_per_period > 0, SolPatronError::InvalidTierCost);
        require!(
            period_duration_seconds >= SECONDS_PER_DAY,
            SolPatronError::InvalidPeriodDuration
        );
        require!(tier_name.len() <= 32, SolPatronError::TierNameTooLong);

        let tier = &mut ctx.accounts.tier;
        tier.creator_profile = ctx.accounts.creator_profile.key();
        tier.tier_id = tier_id;
        tier.cost_per_period = cost_per_period;
        tier.period_duration_seconds = period_duration_seconds;
        tier.is_active = true;
        tier.total_subscribers = 0;
        tier.name = tier_name;
        tier.bump = ctx.bumps.tier;

        let creator_profile = &mut ctx.accounts.creator_profile;
        creator_profile.tier_count = creator_profile.tier_count.checked_add(1).unwrap();
        Ok(())
    }

    /// User subscribes to a tier by funding an escrow PDA for a set number of periods.
    pub fn subscribe(
        ctx: Context<Subscribe>,
        periods_to_fund: u64,
    ) -> Result<()> {
        require!(periods_to_fund > 0, SolPatronError::InvalidPeriodCount);
        let tier = &ctx.accounts.tier;
        require!(tier.is_active, SolPatronError::TierInactive);

        let total_cost = tier
            .cost_per_period
            .checked_mul(periods_to_fund)
            .ok_or(SolPatronError::NumericalOverflow)?;

        // Transfer funds from subscriber to vault
        let transfer_accounts = Transfer {
            from: ctx.accounts.subscriber_token_account.to_account_info(),
            to: ctx.accounts.escrow_vault.to_account_info(),
            authority: ctx.accounts.subscriber.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            transfer_accounts,
        );
        token::transfer(cpi_ctx, total_cost)?;

        let clock = Clock::get()?;
        let now = clock.unix_timestamp;
        let subscription = &mut ctx.accounts.subscription;
        
        let additional_time = tier
            .period_duration_seconds
            .checked_mul(periods_to_fund as i64)
            .ok_or(SolPatronError::NumericalOverflow)?;

        if subscription.is_active && subscription.valid_until_ts > now {
            subscription.valid_until_ts = subscription
                .valid_until_ts
                .checked_add(additional_time)
                .ok_or(SolPatronError::NumericalOverflow)?;
        } else {
            subscription.subscriber = ctx.accounts.subscriber.key();
            subscription.tier = tier.key();
            subscription.creator = ctx.accounts.creator_profile.key();
            subscription.start_ts = now;
            subscription.last_settled_ts = now;
            subscription.valid_until_ts = now
                .checked_add(additional_time)
                .ok_or(SolPatronError::NumericalOverflow)?;
            subscription.is_active = true;
            subscription.bump = ctx.bumps.subscription;
            subscription.vault_bump = ctx.bumps.escrow_vault;

            let creator_profile = &mut ctx.accounts.creator_profile;
            creator_profile.active_subscribers = creator_profile
                .active_subscribers
                .checked_add(1)
                .unwrap();
        }

        subscription.escrow_balance = subscription
            .escrow_balance
            .checked_add(total_cost)
            .ok_or(SolPatronError::NumericalOverflow)?;

        Ok(())
    }

    /// Autonomous keeper or creator settles vested funds from the escrow vault.
    pub fn settle_subscription(ctx: Context<SettleSubscription>) -> Result<()> {
        let subscription = &mut ctx.accounts.subscription;
        require!(subscription.is_active, SolPatronError::SubscriptionInactive);

        let clock = Clock::get()?;
        let now = clock.unix_timestamp;
        let end_point = now.min(subscription.valid_until_ts);

        require!(
            end_point > subscription.last_settled_ts,
            SolPatronError::NothingToSettle
        );

        let duration_to_settle = end_point
            .checked_sub(subscription.last_settled_ts)
            .ok_or(SolPatronError::NumericalOverflow)?;

        let tier = &ctx.accounts.tier;
        let rate_per_second = (tier.cost_per_period as u128)
            .checked_mul(1_000_000_000)
            .unwrap()
            .checked_div(tier.period_duration_seconds as u128)
            .ok_or(SolPatronError::NumericalOverflow)?;

        let raw_payable = (rate_per_second
            .checked_mul(duration_to_settle as u128)
            .unwrap()
            .checked_div(1_000_000_000)
            .unwrap()) as u64;

        let payable_amount = raw_payable.min(subscription.escrow_balance);
        require!(payable_amount > 0, SolPatronError::NothingToSettle);

        let fee_amount = payable_amount
            .checked_mul(PROTOCOL_FEE_BPS)
            .unwrap()
            .checked_div(BPS_DENOMINATOR)
            .unwrap();

        let creator_net = payable_amount
            .checked_sub(fee_amount)
            .ok_or(SolPatronError::NumericalOverflow)?;

        // Transfer Protocol Fee
        let sub_key = subscription.key();
        let seeds = &[
            b"vault",
            sub_key.as_ref(),
            &[subscription.vault_bump],
        ];
        let signer = &[&seeds[..]];

        if fee_amount > 0 {
            let fee_transfer = Transfer {
                from: ctx.accounts.escrow_vault.to_account_info(),
                to: ctx.accounts.protocol_fee_token_account.to_account_info(),
                authority: ctx.accounts.escrow_vault.to_account_info(),
            };
            let fee_cpi = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                fee_transfer,
                signer,
            );
            token::transfer(fee_cpi, fee_amount)?;
        }

        // Transfer Creator Share
        let creator_transfer = Transfer {
            from: ctx.accounts.escrow_vault.to_account_info(),
            to: ctx.accounts.creator_token_account.to_account_info(),
            authority: ctx.accounts.escrow_vault.to_account_info(),
        };
        let creator_cpi = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            creator_transfer,
            signer,
        );
        token::transfer(creator_cpi, creator_net)?;

        // Update state
        subscription.escrow_balance = subscription
            .escrow_balance
            .checked_sub(payable_amount)
            .ok_or(SolPatronError::NumericalOverflow)?;
        subscription.last_settled_ts = end_point;

        if now >= subscription.valid_until_ts && subscription.escrow_balance == 0 {
            subscription.is_active = false;
        }

        let creator_profile = &mut ctx.accounts.creator_profile;
        creator_profile.total_revenue = creator_profile
            .total_revenue
            .checked_add(creator_net)
            .unwrap();

        Ok(())
    }

    /// Allows subscriber to cancel early and refund unvested balance.
    pub fn cancel_subscription(ctx: Context<CancelSubscription>) -> Result<()> {
        let subscription = &mut ctx.accounts.subscription;
        require!(subscription.is_active, SolPatronError::SubscriptionInactive);

        let clock = Clock::get()?;
        let now = clock.unix_timestamp;

        // Perform final settlement up to current time
        let duration_to_settle = (now.min(subscription.valid_until_ts))
            .checked_sub(subscription.last_settled_ts)
            .unwrap_or(0);

        let tier = &ctx.accounts.tier;
        let mut payable_amount: u64 = 0;
        if duration_to_settle > 0 {
            let rate_per_second = (tier.cost_per_period as u128)
                .checked_mul(1_000_000_000)
                .unwrap()
                .checked_div(tier.period_duration_seconds as u128)
                .unwrap();

            payable_amount = ((rate_per_second
                .checked_mul(duration_to_settle as u128)
                .unwrap()
                .checked_div(1_000_000_000)
                .unwrap()) as u64)
                .min(subscription.escrow_balance);
        }

        let refund_amount = subscription
            .escrow_balance
            .checked_sub(payable_amount)
            .ok_or(SolPatronError::NumericalOverflow)?;

        let sub_key = subscription.key();
        let seeds = &[
            b"vault",
            sub_key.as_ref(),
            &[subscription.vault_bump],
        ];
        let signer = &[&seeds[..]];

        // Settle vested remainder to creator
        if payable_amount > 0 {
            let fee_amount = payable_amount
                .checked_mul(PROTOCOL_FEE_BPS)
                .unwrap()
                .checked_div(BPS_DENOMINATOR)
                .unwrap();
            let creator_net = payable_amount.checked_sub(fee_amount).unwrap();

            let creator_transfer = Transfer {
                from: ctx.accounts.escrow_vault.to_account_info(),
                to: ctx.accounts.creator_token_account.to_account_info(),
                authority: ctx.accounts.escrow_vault.to_account_info(),
            };
            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    creator_transfer,
                    signer,
                ),
                creator_net,
            )?;
        }

        // Return unearned funds to subscriber
        if refund_amount > 0 {
            let refund_transfer = Transfer {
                from: ctx.accounts.escrow_vault.to_account_info(),
                to: ctx.accounts.subscriber_token_account.to_account_info(),
                authority: ctx.accounts.escrow_vault.to_account_info(),
            };
            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    refund_transfer,
                    signer,
                ),
                refund_amount,
            )?;
        }

        subscription.is_active = false;
        subscription.escrow_balance = 0;
        subscription.valid_until_ts = now;

        let creator_profile = &mut ctx.accounts.creator_profile;
        if creator_profile.active_subscribers > 0 {
            creator_profile.active_subscribers = creator_profile
                .active_subscribers
                .checked_sub(1)
                .unwrap();
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Account Contexts & State Layouts
// ---------------------------------------------------------------------------

#[derive(Accounts)]
pub struct InitializeProtocol<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + ProtocolConfig::LEN,
        seeds = [b"protocol_config"],
        bump
    )]
    pub config: Account<'info, ProtocolConfig>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeCreator<'info> {
    #[account(
        init,
        payer = creator,
        space = 8 + CreatorProfile::LEN,
        seeds = [b"creator", creator.key().as_ref()],
        bump
    )]
    pub creator_profile: Account<'info, CreatorProfile>,
    #[account(
        mut,
        seeds = [b"protocol_config"],
        bump = config.bump
    )]
    pub config: Account<'info, ProtocolConfig>,
    pub payment_mint: Account<'info, Mint>,
    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(tier_id: u8)]
pub struct CreateTier<'info> {
    #[account(
        init,
        payer = creator,
        space = 8 + SubscriptionTier::LEN,
        seeds = [b"tier", creator_profile.key().as_ref(), &[tier_id]],
        bump
    )]
    pub tier: Account<'info, SubscriptionTier>,
    #[account(
        mut,
        has_one = creator_authority @ SolPatronError::Unauthorized
    )]
    pub creator_profile: Account<'info, CreatorProfile>,
    pub creator_authority: Signer<'info>,
    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Subscribe<'info> {
    #[account(
        init_if_needed,
        payer = subscriber,
        space = 8 + Subscription::LEN,
        seeds = [b"subscription", tier.key().as_ref(), subscriber.key().as_ref()],
        bump
    )]
    pub subscription: Account<'info, Subscription>,
    #[account(
        init_if_needed,
        payer = subscriber,
        seeds = [b"vault", subscription.key().as_ref()],
        bump,
        token::mint = payment_mint,
        token::authority = escrow_vault
    )]
    pub escrow_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub creator_profile: Account<'info, CreatorProfile>,
    pub tier: Account<'info, SubscriptionTier>,
    pub payment_mint: Account<'info, Mint>,
    #[account(
        mut,
        constraint = subscriber_token_account.owner == subscriber.key(),
        constraint = subscriber_token_account.mint == payment_mint.key()
    )]
    pub subscriber_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub subscriber: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SettleSubscription<'info> {
    #[account(
        mut,
        seeds = [b"subscription", tier.key().as_ref(), subscription.subscriber.as_ref()],
        bump = subscription.bump
    )]
    pub subscription: Account<'info, Subscription>,
    #[account(
        mut,
        seeds = [b"vault", subscription.key().as_ref()],
        bump = subscription.vault_bump
    )]
    pub escrow_vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"tier", creator_profile.key().as_ref(), &[tier.tier_id]],
        bump = tier.bump
    )]
    pub tier: Account<'info, SubscriptionTier>,
    #[account(mut)]
    pub creator_profile: Account<'info, CreatorProfile>,
    #[account(
        mut,
        constraint = creator_token_account.owner == creator_profile.creator_authority
    )]
    pub creator_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = protocol_fee_token_account.owner == config.fee_recipient
    )]
    pub protocol_fee_token_account: Account<'info, TokenAccount>,
    #[account(seeds = [b"protocol_config"], bump = config.bump)]
    pub config: Account<'info, ProtocolConfig>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CancelSubscription<'info> {
    #[account(
        mut,
        has_one = subscriber @ SolPatronError::Unauthorized,
        seeds = [b"subscription", tier.key().as_ref(), subscriber.key().as_ref()],
        bump = subscription.bump
    )]
    pub subscription: Account<'info, Subscription>,
    #[account(
        mut,
        seeds = [b"vault", subscription.key().as_ref()],
        bump = subscription.vault_bump
    )]
    pub escrow_vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"tier", creator_profile.key().as_ref(), &[tier.tier_id]],
        bump = tier.bump
    )]
    pub tier: Account<'info, SubscriptionTier>,
    #[account(mut)]
    pub creator_profile: Account<'info, CreatorProfile>,
    #[account(
        mut,
        constraint = creator_token_account.owner == creator_profile.creator_authority
    )]
    pub creator_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = subscriber_token_account.owner == subscriber.key()
    )]
    pub subscriber_token_account: Account<'info, TokenAccount>,
    pub subscriber: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

// ---------------------------------------------------------------------------
// Schemas
// ---------------------------------------------------------------------------

#[account]
pub struct ProtocolConfig {
    pub admin: Pubkey,
    pub fee_recipient: Pubkey,
    pub fee_bps: u64,
    pub total_creators: u64,
    pub bump: u8,
}
impl ProtocolConfig {
    pub const LEN: usize = 32 + 32 + 8 + 8 + 1;
}

#[account]
pub struct CreatorProfile {
    pub creator_authority: Pubkey,
    pub payment_mint: Pubkey,
    pub metadata_uri: String,
    pub tier_count: u8,
    pub active_subscribers: u64,
    pub total_revenue: u64,
    pub bump: u8,
}
impl CreatorProfile {
    pub const LEN: usize = 32 + 32 + (4 + 200) + 1 + 8 + 8 + 1;
}

#[account]
pub struct SubscriptionTier {
    pub creator_profile: Pubkey,
    pub tier_id: u8,
    pub cost_per_period: u64,
    pub period_duration_seconds: i64,
    pub is_active: bool,
    pub total_subscribers: u64,
    pub name: String,
    pub bump: u8,
}
impl SubscriptionTier {
    pub const LEN: usize = 32 + 1 + 8 + 8 + 1 + 8 + (4 + 32) + 1;
}

#[account]
pub struct Subscription {
    pub subscriber: Pubkey,
    pub tier: Pubkey,
    pub creator: Pubkey,
    pub escrow_balance: u64,
    pub start_ts: i64,
    pub last_settled_ts: i64,
    pub valid_until_ts: i64,
    pub is_active: bool,
    pub bump: u8,
    pub vault_bump: u8,
}
impl Subscription {
    pub const LEN: usize = 32 + 32 + 32 + 8 + 8 + 8 + 8 + 1 + 1 + 1;
}

#[error_code]
pub mod SolPatronError {
    #[msg("Metadata URI exceeds 200 bytes limit")]
    MetadataUriTooLong,
    #[msg("Subscription tier cost must be strictly greater than 0")]
    InvalidTierCost,
    #[msg("Period duration must be at least 1 day (86400 seconds)")]
    InvalidPeriodDuration,
    #[msg("Tier name exceeds 32 characters limit")]
    TierNameTooLong,
    #[msg("Number of funding periods must be greater than zero")]
    InvalidPeriodCount,
    #[msg("Subscription tier is disabled or inactive")]
    TierInactive,
    #[msg("Subscription is not active")]
    SubscriptionInactive,
    #[msg("No unlocked escrow funds are available to settle")]
    NothingToSettle,
    #[msg("Arithmetic overflow or underflow detected")]
    NumericalOverflow,
    #[msg("Signer does not possess required authority")]
    Unauthorized,
}