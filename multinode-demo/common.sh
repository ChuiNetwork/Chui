# |source| this file
#
# Common utilities shared by other scripts in this directory
#
# The following directive disable complaints about unused variables in this
# file:
# shellcheck disable=2034
#

# shellcheck source=net/common.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. || exit 1; pwd)"/net/common.sh

prebuild=
if [[ $1 = "--prebuild" ]]; then
  prebuild=true
fi

if [[ $(uname) != Linux ]]; then
    # Protect against unsupported configurations to prevent non-obvious errors
    # later. Arguably these should be fatal errors but for now prefer tolerance.
    if [[ -n $SOLANA_CUDA ]]; then
        echo "Warning: CUDA is not supported on $(uname)"
        SOLANA_CUDA=
    fi
fi

if [[ -n $USE_INSTALL || ! -f "$SOLANA_ROOT"/Cargo.toml ]]; then
    chui_program() {
        declare program="$1"
        if [[ -z $program ]]; then
            printf "chui"
        else
            printf "chui-%s" "$program"
        fi
    }
else
  chui_program() {
    declare program="$1"
    declare crate="$program"
    if [[ -z $program ]]; then
      crate="cli"
      program="chui"
    else
      program="chui-$program"
    fi

    if [[ -n $NDEBUG ]]; then
      maybe_release=--release
    fi

    # Prebuild binaries so that CI sanity check timeout doesn't include build time
    if [[ $prebuild ]]; then
      (
        set -x
        # shellcheck disable=SC2086 # Don't want to double quote
        cargo $CARGO_TOOLCHAIN build $maybe_release --bin $program
      )
    fi

    printf "cargo $CARGO_TOOLCHAIN run $maybe_release  --bin %s %s -- " "$program"
  }
fi

chui_bench_tps=$(chui_program bench-tps)
chui_faucet=$(chui_program faucet solana)
chui_validator=$(chui_program validator)
chui_validator_cuda="$chui_validator --cuda"
chui_genesis=$(chui_program genesis solana)
chui_gossip=$(chui_program gossip)
chui_keygen=$(chui_program keygen)
chui_ledger_tool=$(chui_program ledger-tool)
chui_cli=$(chui_program)

export RUST_BACKTRACE=1

default_arg() {
    declare name=$1
    declare value=$2
    
    for arg in "${args[@]}"; do
        if [[ $arg = "$name" ]]; then
            return
        fi
    done
    
    if [[ -n $value ]]; then
        args+=("$name" "$value")
    else
        args+=("$name")
    fi
}

replace_arg() {
    declare name=$1
    declare value=$2
    
    default_arg "$name" "$value"
    
    declare index=0
    for arg in "${args[@]}"; do
        index=$((index + 1))
        if [[ $arg = "$name" ]]; then
            args[$index]="$value"
        fi
    done
}
