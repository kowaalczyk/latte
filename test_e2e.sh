set -euo pipefail
IFS=$'\n\t'


export LLVM_ASSEMBLER=/usr/local/opt/llvm/bin/llvm-as
export LLVM_LINKER=/usr/local/opt/llvm/bin/llvm-link
export LLVM_INTERPRETER=/usr/local/opt/llvm/bin/lli

# compile the compiler
cargo build --package latc_llvm --bin latc_llvm
cp target/debug/latc_llvm ./

for test_in in tests/good/*.lat; do
  logfile="${test_in%.lat}.log"
  infile="${test_in%.lat}.input"
  compiled="${test_in%.lat}.bc"
  realout="${test_in%.lat}.realout"
  expout="${test_in%.lat}.output"

  # compile program
  set +e
  out=$(./latc_llvm "$test_in" >"$logfile" 2>&1)
  retval=$?
  set -e

  if [[ $retval -ne 0 ]]; then
    echo "COMPILATION ERROR $test_in"
    continue
  fi

  echo "" >> "$logfile"
  echo " --- EXECUTION STDERR --- " >> "$logfile"

  # execute compiled program
  set +e
  if [[ -e "$infile" ]]; then
    out=$(lli "$compiled" < "$infile" > "$realout" 2>> "$logfile")
  else
    out=$(lli "$compiled" > "$realout" 2>> "$logfile")
  fi
  retval=$?
  set -e

  if [[ $retval -ne 0 ]]; then
    echo "RUNTIME ERROR $test_in"
    continue
  fi

  # check program output
  set +e
  out=$(diff "$realout" "$expout")
  retval=$?
  set -e

  if [[ $retval -ne 0 ]]; then
    echo "OUTPUT ERROR $test_in"
    continue
  fi

  rm "$logfile"
  echo "."
done
