# ------------------
# Internal variables
dummy1 := $(shell mkdir bin 2> /dev/null)

# ------------------
# Primary targets
all: bin/gnos

# gnos doesn't return so we start the client before the browser.
run: bin/gnos
	git web--browse 'http://localhost:8080'
	export RUST_LOG=gnos=2,rwebserve=2,socket=1 && export GNOS_USER && ./bin/gnos --admin --root=html scripts/sat.json

run-db: bin/gnos
	git web--browse 'http://localhost:8080'
	export RUST_LOG=gnos=2,rwebserve=1,socket=1,rrdf=0 && export GNOS_USER && ./bin/gnos --admin --root=html --db scripts/sat.json

run-snmp:
	scp scripts/sat.json scripts/snmp-modeler.py jjones@10.8.0.149: && ssh jjones@10.8.0.149 "python snmp-modeler.py -vvv --stdout  --duration=1 sat.json"

check: bin/test-gnos
	./bin/test-gnos

# You can either use this target (assuming that the libraries are in /usr/local/lib/rust)
# or install them via cargo.
update-libraries:
	cp /usr/local/lib/rust/libmustache-*-0.1.dylib bin
	cp /usr/local/lib/rust/libsocket-*-0.1.dylib bin
	cp /usr/local/lib/rust/librparse-*-0.5.dylib bin
	cp /usr/local/lib/rust/librrdf-*-0.2.dylib bin
	cp /usr/local/lib/rust/librwebserve-*-0.1.dylib bin
	rm -f bin/gnos

dist:
	tar --create --compress --exclude \*/.git --exclude \*/.git/\* --file=gnos-0.1.tar.gz \
		Makefile html src

# ------------------
# Binary targets 
bin/gnos: src/gnos.rc src/*.rs src/handlers/*.rs
	rustc -L bin -o $@ $<

bin/test-gnos: src/gnos.rc src/*.rs src/handlers/*.rs
	rustc -L bin --test -o $@ $<
