# ------------------
# Internal variables
dummy1 := $(shell mkdir bin 2> /dev/null)

# ------------------
# Primary targets
all: bin/gnos

# We bind to localhost so that we can hit it using a browser and a 10.x address
# so that machines on the other side of the VPN can send PUTs (we can't just
# bind to the 10.x address because traffic sent there is not locally delivered).
#
# gnos doesn't return so we start the client before the browser.
run: bin/gnos
	git web--browse 'http://localhost:8080'
	export RUST_LOG=gnos=3,rwebserve::server=3,socket=1 && ./bin/gnos --admin --root=html --address=localhost --address=10.6.210.132 --port=8080

run-snmp:
	scp scripts/sat.json jjones@10.8.0.149: && scp scripts/snmp-modeler.py jjones@10.8.0.149: && ssh jjones@10.8.0.149 "python snmp-modeler.py -vvv --stdout  --duration=1 sat.json"

check: bin/test-gnos
	./bin/test-gnos

hello: bin/hello
	export RUST_LOG=hello && ./bin/$@

#export RUST_LOG=::rt::backtrace && ./bin/$@
kata2: bin/kata2
	./bin/$@

# You can either use this target (assuming that the libraries are in /usr/local/lib/rust)
# or install them via cargo.
update-libraries:
	cp /usr/local/lib/rust/libmustache-*-0.1.dylib bin
	cp /usr/local/lib/rust/libsocket-*-0.1.dylib bin
	cp /usr/local/lib/rust/librparse-*-0.3.dylib bin
	cp /usr/local/lib/rust/librrdf-*-0.1.dylib bin
	cp /usr/local/lib/rust/librwebserve-*-0.1.dylib bin
	rm bin/gnos

dist:
	tar --create --compress --exclude \*/.git --exclude \*/.git/\* --file=gnos-0.1.tar.gz \
		Makefile html src

# ------------------
# Binary targets 
bin/gnos: src/gnos.rc src/*.rs
	rustc -g -L bin -o $@ $<

bin/test-gnos: src/gnos.rc src/*.rs
	rustc -g -L bin --test -o $@ $<

bin/hello: katas/hello.rs
	rustc -o $@ $<

bin/kata2: katas/kata2.rs
	rustc -g --test -o $@ $<
