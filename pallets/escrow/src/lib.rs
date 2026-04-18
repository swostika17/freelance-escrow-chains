#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[frame::pallet]
pub mod pallet {
    use alloc::vec::Vec;
    use frame::deps::frame_support::pallet_prelude::*;
    use frame::deps::frame_system;
    use frame::deps::frame_system::pallet_prelude::*;

    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum EscrowStatus {
        Created,
        Submitted,
        Completed,
        Cancelled,
    }

    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct EscrowDetails<AccountId, BlockNumber> {
        pub client: AccountId,
        pub freelancer: AccountId,
        pub amount: u128,
        pub deadline: BlockNumber,
        pub work_hash: BoundedVec<u8, ConstU32<100>>,
        pub status: EscrowStatus,
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>>
            + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::storage]
    #[pallet::getter(fn next_escrow_id)]
    pub type NextEscrowId<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn escrows)]
    pub type Escrows<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u64,
        EscrowDetails<T::AccountId, BlockNumberFor<T>>,
        OptionQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        EscrowCreated {
            escrow_id: u64,
            client: T::AccountId,
            freelancer: T::AccountId,
            amount: u128,
        },
        WorkSubmitted {
            escrow_id: u64,
            freelancer: T::AccountId,
            work_hash: BoundedVec<u8, ConstU32<100>>,
        },
        EscrowApproved {
            escrow_id: u64,
            client: T::AccountId,
        },
        EscrowCancelled {
            escrow_id: u64,
            client: T::AccountId,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        InvalidAmount,
        WorkHashTooLong,
        EscrowNotFound,
        NotClient,
        NotFreelancer,
        InvalidStatus,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(10_000)]
        pub fn create_escrow(
            origin: OriginFor<T>,
            freelancer: T::AccountId,
            amount: u128,
            deadline: BlockNumberFor<T>,
            work_hash: Vec<u8>,
        ) -> DispatchResult {
            let client = ensure_signed(origin)?;

            ensure!(amount > 0, Error::<T>::InvalidAmount);

            let bounded_hash: BoundedVec<u8, ConstU32<100>> =
                work_hash.try_into().map_err(|_| Error::<T>::WorkHashTooLong)?;

            let escrow_id = NextEscrowId::<T>::get();

            let details = EscrowDetails {
                client: client.clone(),
                freelancer: freelancer.clone(),
                amount,
                deadline,
                work_hash: bounded_hash,
                status: EscrowStatus::Created,
            };

            Escrows::<T>::insert(escrow_id, details);
            NextEscrowId::<T>::put(escrow_id + 1);

            Self::deposit_event(Event::EscrowCreated {
                escrow_id,
                client,
                freelancer,
                amount,
            });

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(10_000)]
        pub fn submit_work(
            origin: OriginFor<T>,
            escrow_id: u64,
            work_hash: Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let bounded_hash: BoundedVec<u8, ConstU32<100>> =
                work_hash.try_into().map_err(|_| Error::<T>::WorkHashTooLong)?;

            Escrows::<T>::try_mutate(escrow_id, |maybe_escrow| -> DispatchResult {
                let escrow = maybe_escrow.as_mut().ok_or(Error::<T>::EscrowNotFound)?;

                ensure!(who == escrow.freelancer, Error::<T>::NotFreelancer);
                ensure!(escrow.status == EscrowStatus::Created, Error::<T>::InvalidStatus);

                escrow.work_hash = bounded_hash.clone();
                escrow.status = EscrowStatus::Submitted;

                Ok(())
            })?;

            Self::deposit_event(Event::WorkSubmitted {
                escrow_id,
                freelancer: who,
                work_hash: bounded_hash,
            });

            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(10_000)]
        pub fn approve_work(
            origin: OriginFor<T>,
            escrow_id: u64,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            Escrows::<T>::try_mutate(escrow_id, |maybe_escrow| -> DispatchResult {
                let escrow = maybe_escrow.as_mut().ok_or(Error::<T>::EscrowNotFound)?;

                ensure!(who == escrow.client, Error::<T>::NotClient);
                ensure!(escrow.status == EscrowStatus::Submitted, Error::<T>::InvalidStatus);

                escrow.status = EscrowStatus::Completed;

                Ok(())
            })?;

            Self::deposit_event(Event::EscrowApproved {
                escrow_id,
                client: who,
            });

            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(10_000)]
        pub fn cancel_escrow(
            origin: OriginFor<T>,
            escrow_id: u64,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            Escrows::<T>::try_mutate(escrow_id, |maybe_escrow| -> DispatchResult {
                let escrow = maybe_escrow.as_mut().ok_or(Error::<T>::EscrowNotFound)?;

                ensure!(who == escrow.client, Error::<T>::NotClient);
                ensure!(escrow.status == EscrowStatus::Created, Error::<T>::InvalidStatus);

                escrow.status = EscrowStatus::Cancelled;

                Ok(())
            })?;

            Self::deposit_event(Event::EscrowCancelled {
                escrow_id,
                client: who,
            });

            Ok(())
        }
    }
}

pub use self::pallet::*;