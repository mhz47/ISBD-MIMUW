all:
	cargo build --release --manifest-path src/Cargo.toml

benchmark: all
	python3 benchmark/run_all.py

plots:
	python3 benchmark/plot_all.py

clean:
	cargo clean --manifest-path src/Cargo.toml
	rm -f benchmark/test_data_*.bin
	rm -f benchmark/plots/*.png
	rm -f benchmark/results/*.csv

.PHONY: all benchmark plots clean
