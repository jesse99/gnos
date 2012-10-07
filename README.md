gnos is a Network Management System (NMS) written in [Rust](http://www.rust-lang.org/) and
Javascript using HTTP 1.1 and HTML 5. HTML 5's canvas is used to display a graphical summary 
of the network state. HTML 5's Server-sent events are used to efficiently, and automatically, update 
the web pages as new data comes into the server.

It depends upon:
* The [rwebserve](https://github.com/jesse99/rwebserve) HTTP 1.1 server library.
* The [rrdf](https://github.com/jesse99/rrdf) [Resource Description Language](http://www.w3.org/RDF/) (RDF) library.
* The [runits](https://github.com/jesse99/runits) [SI](http://en.wikipedia.org/wiki/SI) units library.
* The [rust-socket](https://github.com/jdm/rust-socket) library.
* The [rust-mustache](https://github.com/erickt/rust-mustache) template library.
* The [rparse](https://github.com/jesse99/rparse) parser combinator library.
* [jsl](http://www.javascriptlint.com/) is used by the Makefile to perform syntax (and other) checks before running.
* [R](http://www.r-project.org/) is used to generate server-side charts (on Linux install r-base).

Server side testing has been mostly done on a Mac. Linux should work as well. Windows will likely require some 
work (mostly in rust-socket).

Client side testing has been mostly done with Chrome 22 beta (earlier versions of Chrome cap the number
of outstanding server-side events at something silly like four which prevented the browser from displaying all 
the information). 
