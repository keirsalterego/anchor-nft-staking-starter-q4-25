use anchor_lang::prelude::*;
use mpl_core::{
    instructions::{RemovePluginV1CpiBuilder, UpdatePluginV1CpiBuilder},
    types::{FreezeDelegate, Plugin, PluginType},
    ID as CORE_PROGRAM_ID,
};

use crate::{
    errors::StakeError,
    state::{CollectionInfo, StakeAccount, StakeConfig, UserAccount},
};

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut)]
    /// CHECK: Verified by mpl-core
    pub asset: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Verified by mpl-core
    pub collection: UncheckedAccount<'info>,

    #[account(
        mut,
        close = user,
        seeds = [b"stake", config.key().as_ref(), asset.key().as_ref()],
        bump = stake_account.bump,
    )]
    pub stake_account: Account<'info, StakeAccount>,

    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, StakeConfig>,

    #[account(
        mut,
        seeds = [b"user", user.key().as_ref()],
        bump = user_account.bump,
    )]
    pub user_account: Account<'info, UserAccount>,

    #[account(
        seeds = [b"collection_info", collection.key().as_ref()],
        bump = collection_info.bump,
    )]
    pub collection_info: Account<'info, CollectionInfo>,

    #[account(address = CORE_PROGRAM_ID)]
    /// CHECK: Verified by address constraint
    pub core_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> Unstake<'info> {
    pub fn unstake(&mut self) -> Result<()> {
        let current_time = Clock::get()?.unix_timestamp;
        let time_elapsed = current_time.saturating_sub(self.stake_account.staked_at);
        let freeze_period_seconds = (self.config.freeze_period as i64)
            .saturating_mul(86400);

        require!(
            time_elapsed >= freeze_period_seconds,
            StakeError::FreezePeriodNotPassed
        );

        let points_earned = time_elapsed
            .saturating_div(86400)
            .saturating_mul(self.config.points_per_stake as i64);

        self.user_account.points = self
            .user_account
            .points
            .saturating_add(points_earned as u32);

        self.user_account.amount_staked = self.user_account.amount_staked.saturating_sub(1);

        let signer_seeds: &[&[&[u8]]] = &[&[
            b"collection_info",
            &self.collection.key().to_bytes(),
            &[self.collection_info.bump],
        ]];

        UpdatePluginV1CpiBuilder::new(&self.core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.user.to_account_info())
            .authority(Some(&self.collection_info.to_account_info()))
            .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: false }))
            .system_program(&self.system_program.to_account_info())
            .invoke_signed(signer_seeds)?;

        RemovePluginV1CpiBuilder::new(&self.core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.user.to_account_info())
            .authority(Some(&self.user.to_account_info()))
            .plugin_type(PluginType::FreezeDelegate)
            .system_program(&self.system_program.to_account_info())
            .invoke()?;

        Ok(())
    }
}
