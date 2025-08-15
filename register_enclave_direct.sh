#!/bin/bash

# Script để đăng ký enclave mà không cần endpoint HTTP
# Sử dụng NSM driver trực tiếp thông qua CLI tool

# Kiểm tra arguments
if [ "$#" -ne 5 ]; then
    echo "Usage: $0 <enclave_package_id> <examples_package_id> <enclave_config_id> <module_name> <otw_name>"
    echo "Example: $0 0x872852f77545c86a8bd9bdb8adc9e686b8573fc2a0dab0af44864bc1aecdaea9 0x2b70e34684d696a0a2847c793ee1e5b88a23289a7c04dd46249b95a9823367d9 0x86775ced1fdceae31d090cf48a11b4d8e4a613a2d49f657610c0bc287c8f0589 weather WEATHER"
    exit 1
fi

ENCLAVE_PACKAGE_ID=$1
EXAMPLES_PACKAGE_ID=$2
ENCLAVE_CONFIG_OBJECT_ID=$3
MODULE_NAME=$4
OTW_NAME=$5

echo 'Building CLI tool for direct attestation...'
cd src/get_attestation_cli
cargo build --release
cd ../..

echo 'Lấy attestation trực tiếp từ NSM driver...'
# Sử dụng CLI tool để lấy attestation với keypair mới
ATTESTATION_HEX=$(./target/release/get_attestation_cli --generate-keypair | tail -1)

echo "Đã lấy attestation, độ dài=${#ATTESTATION_HEX}"

if [ ${#ATTESTATION_HEX} -eq 0 ]; then
    echo "Lỗi: Attestation trống. Vui lòng kiểm tra NSM driver."
    exit 1
fi

# Chuyển đổi hex thành array sử dụng Python
ATTESTATION_ARRAY=$(python3 - <<EOF
import sys

def hex_to_vector(hex_string):
    byte_values = [str(int(hex_string[i:i+2], 16)) for i in range(0, len(hex_string), 2)]
    rust_array = [f"{byte}u8" for byte in byte_values]
    return f"[{', '.join(rust_array)}]"

print(hex_to_vector("$ATTESTATION_HEX"))
EOF
)

echo 'Đã chuyển đổi attestation'
# Thực thi sui client command
sui client ptb --assign v "vector$ATTESTATION_ARRAY" \
    --move-call "0x2::nitro_attestation::load_nitro_attestation" v @0x6 \
    --assign result \
    --move-call "${ENCLAVE_PACKAGE_ID}::enclave::register_enclave<${EXAMPLES_PACKAGE_ID}::${MODULE_NAME}::${OTW_NAME}>" @${ENCLAVE_CONFIG_OBJECT_ID} result \
    --gas-budget 100000000
