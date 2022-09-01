# trapez

A simple Rust-based transaction processor.

## Implementation

### Modules

The implementation consists of the following parts (from inner to outer):

#### `account`

Main account logic for a single client.

#### `processor`

Maintains per-client accounts and allows for async communication via channels. Transactional commands 
are sent as messages to its command channel and errors are retrieved via the error channel.

State requests are replied to via oneshot channel in the `GetState` message.

#### `cli`

Provides a `run` method which reads CSV records via the writer argument and writes the resulting state
to the reader argument. Errors get written to stderr.

### Currency amount values

Amounts are stored as `i64` throughout as an 1/10000th of a currency unit. This enables storage of negative amounts in the transaction log without any conversions and avoids floating point math. The parsing from and rendering to decimal strings is done as part of CSV (de-)serialization.
