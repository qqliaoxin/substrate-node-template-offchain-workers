#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use sp_runtime::{
    offchain::{
        storage::{StorageValueRef,MutateStorageError,StorageRetrievalError},
    },
    // traits::Zero,
};


#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use sp_std::{collections::vec_deque::VecDeque, prelude::*, str};
	use frame_support::inherent::Vec;

	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	use sp_io::offchain_index;
	use serde::{Deserialize, Deserializer};
	use hex_literal::hex;
	use sp_core::{
		crypto::Public as _,
		H256,
		H512,
		sr25519::{Public, Signature},
	};

	const ONCHAIN_TX_KEY: &[u8] = b"pallet::offchain-indexing::";

	#[derive(Debug, Deserialize, Encode, Decode, Default)]
	struct IndexingData(Vec<u8>, u64);
	
	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	// The pallet's runtime storage items.
	// https://docs.substrate.io/main-docs/build/runtime-storage/
	#[pallet::storage]
	#[pallet::getter(fn something)]
	// Learn more about declaring storage items:
	// https://docs.substrate.io/main-docs/build/runtime-storage/#declaring-storage-items
	pub type Something<T> = StorageValue<_, u32>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored { something: u32, who: T::AccountId },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::call_index(0)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn do_something(origin: OriginFor<T>, something: u32) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/main-docs/build/origins/
			let who = ensure_signed(origin)?;

			// Update storage.
			<Something<T>>::put(something);

			// Emit an event.
			Self::deposit_event(Event::SomethingStored { something, who });
			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}

		/// An example dispatchable that may throw a custom error.
		#[pallet::call_index(1)]
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1).ref_time())]
		pub fn cause_error(origin: OriginFor<T>) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// Read a value from storage.
			match <Something<T>>::get() {
				// Return an error if the value has not been set.
				None => return Err(Error::<T>::NoneValue.into()),
				Some(old) => {
					// Increment the value read from storage; will error in the event of overflow.
					let new = old.checked_add(1).ok_or(Error::<T>::StorageOverflow)?;
					// Update the value in storage with the incremented result.
					<Something<T>>::put(new);
					Ok(())
				},
			}
		}
		#[pallet::call_index(2)]
		#[pallet::weight(100)]
		pub fn offchain_index_extrinsic(origin: OriginFor<T>, number: u64) -> DispatchResult {
			// let who = ensure_signed(origin)?;
			let key = Self::derived_key(frame_system::Module::<T>::block_number());
			// log::info!("=== offchain_index_key ===>> {:?}",key.as_slice());
			let buffer = &key[..];
			let pring =  H256::from_slice(buffer);
			log::info!("=== offchain_index_key ===>> {:?}",pring);
			let data = IndexingData(b"submit_number_unsigned".to_vec(), number);
			// 写入自定义索引数据
			offchain_index::set(&key, &data.encode());
			Ok(())
		}
	}
	
	// https://docs.substrate.io/reference/how-to-guides/offchain-workers/
	// https://github.com/JoshOrndorff/recipes/blob/03b7a0657727705faa5f840c73bcf15ffdd81f2b/pallets/ocw-demo/src/lib.rs#L207
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(block_number: T::BlockNumber) {
			// Reading back the offchain indexing value. This is exactly the same as reading from
			// ocw local storage.
			let key = Self::derived_key(block_number - 1u32.into());
			let storage_ref = StorageValueRef::persistent(&key);
		
			if let Ok(Some(data)) = storage_ref.get::<IndexingData>() {
				log::info!("local storage data: {:?}, {:?}",
					str::from_utf8(&data.0).unwrap_or("error"), data.1);
			} else {
				log::info!("Error reading from local storage.");
			}
		}
		// fn offchain_worker(block_number: T::BlockNumber) {
		// 	log::info!("=== hooks OCW === >>  offchain worker block:: {:?}", block_number);
		// 	//offchain-local-storage
		// 	let key = Self::derived_key(block_number);
		// 	// let  storage = StorageValueRef::persistent(b"pallet::ocw-storage");
		// 	let storage = StorageValueRef::persistent(&key);
		// 	// 写入值 
		// 	//  get a local random value 
		// 	let random_slice = sp_io::offchain::random_seed();
		// 	//  get a local timestamp
		// 	let timestamp_u64 = sp_io::offchain::timestamp().unix_millis();
		// 	// combine to a tuple and print it  
		// 	let value = (random_slice, timestamp_u64);
		// 	// log::info!("=== hooks OCW === >>  in block, local storage to write: {:?}", value);

		// 	// 使用 mutate 修改 storage 原子数据，交写入
		// 	struct StateError;
		// 	//  write or mutate tuple content to key
		// 	let res = storage.mutate(|val: Result<Option<([u8;32], u64)>, StorageRetrievalError>| -> Result<_, StateError> {
		// 		match val {
		// 			Ok(Some(_)) => Ok(value),
		// 			_ => Ok(value),
		// 		}
		// 	});

		// 	match res {
		// 		Ok(value) => {
		// 			log::info!("=== hooks OCW === >> in block, mutate successfully: {:?}", value);
		// 		},
		// 		Err(MutateStorageError::ValueFunctionFailed(_)) => (),
		// 		Err(MutateStorageError::ConcurrentModification(_)) => (),
		// 	}

		// 	// 写入 loca storage 数据， write or mutate tuple content to key
		// 	// storage.set(&value);

		// 	// 检查存储是否包含缓存值。
		// 	// if let Ok(Some(res)) = storage.get::<([u8;32], u64)>() {
		// 	// 	log::info!("=== hooks OCW === >>  cached result: {:?}", res);
		// 	// 	// delete that key
		// 	// 	// storage.clear();
		// 	// }
		// }
	}
	impl<T: Config> Pallet<T> {
        #[deny(clippy::clone_double_ref)]
        fn derived_key(block_number: T::BlockNumber) -> Vec<u8> {
            block_number.using_encoded(|encoded_bn| {
                // b"node-template::storage::"
				//.iter()
				ONCHAIN_TX_KEY.clone().into_iter()
				    .chain(b"/".into_iter())
                    .chain(encoded_bn)
                    .copied()
                    .collect::<Vec<u8>>()
            })
        }
    }
}
