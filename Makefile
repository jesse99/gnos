# ------------------
# Internal variables
dummy1 := $(shell mkdir bin 2> /dev/null)

RUSTC ?= rustc
SCP ?= scp

ifeq ($(strip $(shell command -v jsl)),)
	JSL = true 'skipping jsl lint phase'
else
	JSL ?= jsl
endif

# ------------------
# Primary targets
all: bin/gnos check-js

# gnos doesn't return so we start the client before the browser.
run: bin/gnos check-js
	export RUST_LOG=gnos=2,rwebserve=2,socket=1,::rt::backtrace && export GNOS_USER && ./bin/gnos --admin --root=html --browse='http://localhost:8080' scripts/fat.json
	#export RUST_LOG=gnos=2,rwebserve=2,socket=1 && export GNOS_USER && ./bin/gnos --admin --root=html scripts/sat.json

run-db: bin/gnos check-js
	export RUST_LOG=gnos=2,rwebserve=1,socket=1,rrdf=0 && export GNOS_USER && ./bin/gnos --admin --root=html --db scripts/fat.json --browse='http://localhost:8080'

run-snmp:
	$(SCP) scripts/fat.json scripts/snmp-modeler.py jjones@10.8.0.179: && ssh jjones@10.8.0.179 "python snmp-modeler.py -vvv --stdout  --dont-put --duration=1 mini-fat.json"
	#$(SCP) scripts/sat.json scripts/snmp-modeler.py jjones@10.8.0.149: && ssh jjones@10.8.0.149 "python snmp-modeler.py -vvv --stdout  --duration=1 sat.json"
	
check-js: html/javascript/*.js html/javascript/scene/*.js
	$(JSL) -nologo -nofilelisting -nocontext -conf jsl.conf -process 'html/javascript/*.js' -process 'html/javascript/scene/*.js'
	
check: bin/test-gnos
	export RUST_LOG=gnos=1,rwebserve=1,socket=1,rrdf=0 && ./bin/test-gnos

check1: bin/test-gnos
	export RUST_LOG=gnos=2,rwebserve=1,socket=1,rrdf=0 && ./bin/test-gnos test_query

# You can either use this target (assuming that the libraries are in /usr/local/lib/rust)
# or install them via cargo.
update-libraries:
	cp /usr/local/lib/rust/libmustache-*-0.3pre.* bin
	cp /usr/local/lib/rust/libsocket-*-0.1.* bin
	cp /usr/local/lib/rust/librparse-*-0.6.* bin
	cp /usr/local/lib/rust/librrdf-*-0.2.* bin
	cp /usr/local/lib/rust/librunits-*-0.1.* bin
	cp /usr/local/lib/rust/librwebserve-*-0.2.* bin
	rm -f bin/gnos

dist:
	tar --create --compress --exclude \*/.git --exclude \*/.git/\* --file=gnos-0.1.tar.gz \
		Makefile html src

# ------------------
# Binary targets 
bin/gnos: src/crate.rc src/*.rs src/handlers/*.rs
	$(RUSTC) -L bin -o $@ $<

bin/test-gnos: src/crate.rc src/*.rs src/handlers/*.rs src/tests/*.rs
	$(RUSTC) -L bin --test -o $@ $<
