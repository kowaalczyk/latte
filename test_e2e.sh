set -euo pipefail
IFS=$'\n\t'


# compile the compiler
cargo build --package latc_llvm --bin latc_llvm
cp target/debug/latc_llvm ./
chmod +x latc_llvm

test_dir="$1"
echo "$test_dir"

failed_cases=0

for test_in in "$test_dir"/*.lat; do
  logfile="${test_in%.lat}.log"
  infile="${test_in%.lat}.input"
  llfile="${test_in%.lat}.ll"
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
    failed_cases=$((failed_cases + 1))
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
    failed_cases=$((failed_cases + 1))
    continue
  fi

  # check program output
  set +e
  out=$(diff "$realout" "$expout")
  retval=$?
  set -e

  if [[ $retval -ne 0 ]]; then
    echo "OUTPUT ERROR $test_in"
    failed_cases=$((failed_cases + 1))
    continue
  fi

  # mark test as passed to indicate progress
  echo "."

  # clean generated files for test cases that passed
done

if [[ $failed_cases -gt 0 ]]; then
  echo "Failed $failed_cases tests"
else
  echo "All tests passed"
fi

exit $failed_cases
