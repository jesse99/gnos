# ------------------
# Internal variables
dummy1 := $(shell mkdir bin 2> /dev/null)

RUSTC ?= rustc
SCP ?= scp
LOCAL_IP ?= utun0

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
	#export RUST_LOG=gnos=2,rwebserve=2,socket=1,::rt::backtrace && export GNOS_USER && ./bin/gnos --admin --root=html --browse='http://localhost:8080' scripts/fat.json
	export RUST_LOG=gnos=2,rwebserve=2,socket=1,::rt::backtrace=4 && export GNOS_USER && ./bin/gnos --admin --root=html --bind=$(LOCAL_IP) --browse='http://localhost:8080' scripts/blos-c2.json

run-net:
	$(SCP) scripts/*.json scripts/*.py jjones@10.4.0.3: && ssh jjones@10.4.0.3 "python net-modeler.py -vvvv --stdout  --dont-put --duration=0 mini-c2.json"

run-invert:
	$(SCP) scripts/*.json scripts/*.py jjones@10.4.0.3: && ssh jjones@10.4.0.3 "python invert-modeler.py -vvvv --stdout  --dont-put --duration=0 blos-c2.json"

run-db: bin/gnos lint-js
	export RUST_LOG=gnos=2,rwebserve=1,socket=1,rrdf=0,::rt::backtrace=4 && export GNOS_USER && ./bin/gnos --admin --root=html --db scripts/fat.json --browse='http://localhost:8080'

profile: bin/gnos lint-js
	export RUST_LOG=gnos=1 && export GNOS_USER && export RUST_MIN_STACK=1048576 && ./bin/gnos --admin --root=html --bind=$(LOCAL_IP) --browse='http://localhost:8080' scripts/blos-c2.json

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
	tar --create --compress --exclude \*/.git --exclude \*/.git/\* --file=gnos-0.2.tar.gz \
		Makefile html src

# ------------------
# Binary targets 
bin/gnos: src/crate.rc src/*.rs src/handlers/*.rs
	$(RUSTC) -L bin -o $@ $<

bin/test-gnos: src/crate.rc src/*.rs src/handlers/*.rs src/tests/*.rs
	$(RUSTC) -L bin --test -o $@ $<
