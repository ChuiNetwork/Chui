#!/usr/bin/env bash
#
# Finds the version of sbf-tools used by this source tree.
#
# stdout of this script may be eval-ed.
#

here="$(dirname "$0")"

SBF_TOOLS_VERSION=unknown

cargo_build_bpf_main="${here}/../sdk/cargo-build-bpf/src/main.rs"
if [[ -f "${cargo_build_bpf_main}" ]]; then
    version=$(sed -e 's/^.*bpf_tools_version\s*=\s*"\(v[0-9.]\+\)".*/\1/;t;d' "${cargo_build_bpf_main}")
    if [[ ${version} != '' ]]; then
        SBF_TOOLS_VERSION="${version}"
    else
        echo '--- unable to parse SBF_TOOLS_VERSION'
    fi
else
    echo "--- '${cargo_build_bpf_main}' not present"
fi

echo SBF_TOOLS_VERSION="${SBF_TOOLS_VERSION}"
