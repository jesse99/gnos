# ------------------
# Internal variables
dummy1 := $(shell mkdir bin 2> /dev/null)

RUSTC ?= rustc
SCP ?= scp

ifeq ($(strip $(shell command -v jsl 2> /dev/null)),)
	JSL = true 'skipping jsl lint phase'
else
	JSL ?= jsl
endif

# ------------------
# Primary targets
all: bin/gnos lint-js

# gnos doesn't return so we start the client before the browser.
run: bin/gnos lint-js
	export RUST_LOG=gnos=2,rwebserve=2,socket=1,::rt::backtrace && export GNOS_USER && ./bin/gnos --admin --root=html --browse='http://localhost:8080' scripts/fat.json

run-db: bin/gnos lint-js
	export RUST_LOG=gnos=2,rwebserve=1,socket=1,rrdf=0 && export GNOS_USER && ./bin/gnos --admin --root=html --db scripts/fat.json --browse='http://localhost:8080'

run-net:
	$(SCP) scripts/*.json scripts/*.py jjones@10.8.0.179: && ssh jjones@10.8.0.179 "python net-modeler.py -vvv --stdout  --dont-put --duration=0 mini-fat.json"

lint-js: html/javascript/*.js html/javascript/scene/*.js
	$(JSL) -nologo -nofilelisting -nocontext -conf jsl.conf -process 'html/javascript/*.js' -process 'html/javascript/scene/*.js'
	
check: bin/test-gnos
	export RUST_LOG=gnos=1,rwebserve=1,socket=1,rrdf=0 && ./bin/test-gnos

check1: bin/test-gnos
	export RUST_LOG=gnos=2,rwebserve=1,socket=1,rrdf=0 && ./bin/test-gnos test_query

check-js: bin/gnos lint-js
	export RUST_LOG=gnos=2,rwebserve=1,socket=1,rrdf=0 && export GNOS_USER && ./bin/gnos --admin --root=html --db scripts/fat.json --browse='http://localhost:8080/test'

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
