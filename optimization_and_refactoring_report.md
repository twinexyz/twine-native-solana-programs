### **Optimization & Refactoring Report: `tokens_gateway` Program**

---

### **High-Impact Recommendations (Gas and Compute)**

#### **H-1: Inefficient Nonce Tracking for Withdrawals and Payouts**

*   **Files:**
    *   `core/state.rs`
    *   `execute_l2_withdrawal/execute_native_l2_withdrawal.rs`
    *   `execute_l2_withdrawal/execute_spl_l2_withdrawal.rs`
    *   `process_refund/process_native_refund.rs`
    *   `process_refund/process_spl_refund.rs`
*   **Observation:** The `ExecutedWithdrawalsBuffer` and `ExecutedPayoutsBuffer` structs use a `Vec<u64>` to store executed nonces. Checking if a nonce has already been executed is done with `executed_withdrawal_nonces.contains()`, which performs a linear scan (O(n) complexity). This will become increasingly slow and expensive as the number of executed transactions grows. Furthermore, the `post_withdrawal_processing` function, which is meant to prune the list, is also inefficient as it involves sorting and cloning the vector.
*   **Recommendation:**
    1.  **Use a Sorted Vector:** Keep the `executed_withdrawal_nonces` and `executed_payout_nonces` vectors sorted at all times.
    2.  **Use Binary Search:** When checking for a nonce, use `binary_search()` instead of `contains()`. This will reduce the complexity from O(n) to O(log n), resulting in a significant reduction in compute units for each withdrawal/refund.
    3.  **Efficient Insertion:** Insert new nonces while maintaining the sorted order. You can find the correct insertion point using `binary_search()` and then use `insert()`.
    4.  **Optimize Pruning:** The `post_withdrawal_processing` function can be made much more efficient by finding the index of the `withdrawal_nonce_lower_bound` and draining up to that index, without needing to sort the entire vector every time.

#### **H-2: Inefficient Use of Strings for Addresses and Amounts**

*   **Files:**
    *   `core/state.rs`
    *   `core/instruction.rs`
    *   Multiple instruction processing files.
*   **Observation:** The program frequently uses `String` to represent L1 and L2 addresses, as well as token amounts. This has several major disadvantages:
    *   **High Gas Cost:** Storing variable-length strings on-chain is much more expensive than storing fixed-size data types.
    *   **High Computational Cost:** Serializing, deserializing, and parsing strings on-chain is computationally intensive. The `decode_*_values` functions in withdrawal/refund logic are a prime example of complex and fragile byte manipulation.
    *   **Increased Risk:** Manual parsing of strings and byte arrays is error-prone.
*   **Recommendation:**
    *   **Addresses:** Replace `String` representations of addresses with fixed-size arrays. For example, a Solana `Pubkey` can be stored as `[u8; 32]` and an Ethereum address as `[u8; 20]`.
    *   **Amounts:** Instead of storing amounts as strings and parsing them, use `u64` or `u128`. For amounts that might exceed `u128`, you can use `[u8; 32]` to represent a `U256`.
    *   **Refactor State & Instructions:** Update the `L1OriginTxPublicValues`, `L2WithdrawValues`, `MessageInfo`, and other relevant structs to use these fixed-size types. This will simplify the instruction logic, reduce the need for on-chain parsing, and significantly lower both gas and compute costs.

---

### **Medium-Impact Recommendations (Robustness and Code Structure)**

#### **M-1: Redundant Code for Token Transfers**

*   **Files:**
    *   `execute_l2_withdrawal/execute_native_l2_withdrawal.rs`
    *   `process_refund/process_native_refund.rs`
    *   `execute_l2_withdrawal/execute_spl_l2_withdrawal.rs`
    *   `process_refund/process_spl_refund.rs`
*   **Observation:** The `process_native_token_withdrawal` function is duplicated across multiple files. The same is true for `process_spl_token_withdrawal`. This violates the DRY (Don't Repeat Yourself) principle and makes the code harder to maintain.
*   **Recommendation:** Create a shared utility module (e.g., `utils/transfers.rs`) and move these functions there. This will centralize the transfer logic, reduce code duplication, and make it easier to apply changes or fixes in one place.

#### **M-2: Manual and Fragile Deserialization of Public Values**

*   **Files:**
    *   `execute_l2_withdrawal/execute_native_l2_withdrawal.rs`
    *   `execute_l2_withdrawal/execute_spl_l2_withdrawal.rs`
    *   `process_refund/process_native_refund.rs`
    *   `process_refund/process_spl_refund.rs`
*   **Observation:** The `decode_*_values` functions perform manual byte-level parsing based on calculated offsets and lengths. This is highly fragile; a small change in the data structure on the client side could break the on-chain program in hard-to-debug ways.
*   **Recommendation:** Define a struct that represents the exact layout of the `public_values` data and use `BorshDeserialize::try_from_slice()` to deserialize it. This is a much safer, more robust, and more efficient way to handle structured data. This is related to recommendation H-2, as using fixed-size types will make this much easier.

---

### **Low-Impact Recommendations (Best Practices)**

#### **L-1: Integer Overflow/Underflow in Vault Data**

*   **File:** `utils/spl_tokens_impl.rs`, `native/native_deposit.rs`
*   **Observation:** The `update_deposit` and `update_withdraw` functions in `spl_tokens_impl.rs` use `checked_add` and `checked_sub`, which is excellent. However, the `native_token_deposit.rs` file also performs a `checked_add` on the `total_deposits` for the native vault. The withdrawal functions for the native vault, however, do not update the `total_deposits` field at all, leading to an inconsistency in the total amount tracked.
*   **Recommendation:** Ensure that both deposits and withdrawals consistently and correctly update the `total_deposits` and `total_deposited_amount` fields for both native and SPL token vaults. Use `checked_add` and `checked_sub` in all cases to prevent overflows and underflows.

#### **L-2: Unclear Error Mapping**

*   **Observation:** The code sometimes uses `ProgramError::InvalidArgument` or `ProgramError::InvalidAccountData` where a more specific custom error from `ProgramCustomError` would be more informative for developers and users.
*   **Recommendation:** Review the error handling and map errors to the most specific custom error possible. For example, instead of a generic `InvalidArgument`, use a more descriptive error like `InvalidTokenAddress` or `AmountCannotBeZero`.

---

### **Summary of Proposed Actions**

1.  **Refactor State:** Change `String`-based fields in state and instruction structs to fixed-size types (`[u8; 32]`, `[u8; 20]`, `u64`, `u128`).
2.  **Optimize Nonce Storage:** Change the `Vec<u64>` for nonces to a sorted vector and use binary search for checks and insertions.
3.  **Centralize Logic:** Move duplicated functions like `process_native_token_withdrawal` into a shared utility module.
4.  **Improve Deserialization:** Replace manual byte parsing with `Borsh` deserialization for public values passed into instructions.
5.  **Review and Refine Error Handling:** Ensure all arithmetic is checked and that errors are specific and meaningful.

By implementing these changes, the `tokens_gateway` program will be more robust, secure, and significantly more efficient, leading to lower gas costs for users and a more maintainable codebase.
