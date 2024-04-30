for i in {0..100000}; do
    ./target/release/keywhisker run-generation "$@" &> data/$i.csv
    printf "$i\r"
done
