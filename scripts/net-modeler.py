#!/usr/bin/python
#import cgi, json, itertools, httplib, re, sys, threading, time
import linux_ssh, snmp
from base_modeler import *

try:
	import argparse
except:
	sys.stderr.write("This script requires Python 2.7 or later\n")
	sys.exit(2)

# Parse command line.
parser = argparse.ArgumentParser(description = "Uses snmp and/or ssh to model a network and sends the result to a gnos server.")
parser.add_argument("--dont-put", dest = 'put', action='store_false', default=True, help = 'log results instead of PUTing them')
parser.add_argument("--duration", action='store', default=float('inf'), type=float, metavar='SECS', help = 'amount of time to poll (for testing)')
parser.add_argument("--stdout", action='store_true', default=False, help = 'log to stdout instead of snmp-modeler.log')
parser.add_argument("--verbose", "-v", action='count', help = 'print extra information')
parser.add_argument("--version", "-V", action='version', version='%(prog)s 0.1')	# TODO: keep this version synced up with the gnos version
parser.add_argument("config", metavar = "CONFIG-FILE", help = "path to json formatted configuration file")
env.options = parser.parse_args()

# Configure logging.
configure_logging(env.options.stdout, 'net-modeler.log')

# Read config info.
with open(env.options.config, 'r') as f:
	env.config = json.load(f)
	
linux_ssh.start()
snmp.start()
