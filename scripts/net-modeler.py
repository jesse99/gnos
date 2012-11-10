#!/usr/bin/python
# Collects information about a network using snmp and ssh. Ships the results off to gnos using json.
import httplib, json, sys, time		# cgi, , itertools, , re, , threading, 
import linux_ssh, snmp
from base_modeler import *

try:
	import argparse
except:
	sys.stderr.write("This script requires Python 2.7 or later\n")
	sys.exit(2)

class Poll(object):
	def __init__(self):
		self.__startTime = time.time()
		self.__last_time = None
		self.__num_updates = 0
		self.__connection = None
		if env.options.put:
			address = "%s:%s" % (env.config['server'], env.config['port'])
			self.__connection = httplib.HTTPConnection(address, strict = True, timeout = 10)
	
	def run(self):
		try:
			if self.__connection:
				send_entities(self.__connection)
			
			rate = env.config['poll-rate']
			while True:
				self.__current_time = time.time()
				if not env.options.put:
					env.logger.info("-" * 60)
					
				data = self.__query()
				send_update(self.__connection, data)
				self.__num_updates += 1
				
				elapsed = time.time() - self.__current_time
				self.__last_time = self.__current_time
				env.logger.info('elapsed: %.1f seconds' % elapsed)
				if time.time() - self.__startTime < env.options.duration:
					time.sleep(max(rate - elapsed, 5))
				else:
					break
		finally:
			if self.__connection:
				self.__connection.close()
				
	def __query(self):
		data = {'modeler': 'net', 'entities': [], 'relations': [], 'labels': [], 'gauges': [], 'details': [], 'alerts': [], 'samples': [], 'charts': []}
		for device in env.config['devices'].values():
			if device['modeler'] == 'linux_ssh':
				query = linux_ssh.QueryDevice(device)
			elif device['modeler'] == 'snmp':
				query = snmp.QueryDevice(device)
			else:
				env.logger.error("bad modeler: %s" % device['modeler'])
				
			if query:
				query.run(data, self.__num_updates)
		return data
		
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
	
poller = Poll()
poller.run()
