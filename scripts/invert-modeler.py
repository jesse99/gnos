#!/usr/bin/python
# BLOS-C2 specific modeler that checks for correct VLAN configuration.
import httplib, json, socket, sys, time
import snmp
from helpers import *
from net_types import *

try:
	import argparse
except:
	sys.stderr.write("This script requires Python 2.7 or later\n")
	sys.exit(2)

class Poll(object):
	def __init__(self):
		self.__startTime = time.time()
		self.__connection = None
		if env.options.put:
			address = "%s:%s" % (env.config['server'], env.config['port'])
			self.__connection = httplib.HTTPConnection(address, strict = True, timeout = 10)
	
	def run(self):
		try:
			rate = env.config['poll-rate']
			while True:
				self.__current_time = time.time()
				if not env.options.put:
					env.logger.info("-" * 60)
					
				devices = map(lambda e: Device(e[0], e[1]), env.config['devices'].items())
				data = {'modeler': 'invert', 'alerts': []}
				self.__query_devices(data, devices)
				send_update(self.__connection, data)
				
				elapsed = time.time() - self.__current_time
				env.logger.info('elapsed: %.1f seconds' % elapsed)
				if time.time() - self.__startTime < env.options.duration:
					time.sleep(max(rate - elapsed, 5))
				else:
					break
		finally:
			if self.__connection:
				self.__connection.close()
				
	def __query_devices(self, data, devices):
		for device in devices:
			if 'invert-modeler' in device.config['modeler']:
				try:
					results = self.__query_device(device)
					self.__process_results(device, data, results)
				except:
					env.logger.error("query_device %s failed" % device.name, exc_info = True)
			
	def __query_device(self, device):
		command = 'snmpbulkget %s %s -Oq -Ot -OU -OX %s' % (device.config['authentication'], device.admin_ip, 'cviRoutedVlanIfIndex ifAlias')
		return run_process(command)
	
	# key = [cviVlanId][cviPhysicalIfIndex]
	# CISCO-VLAN-IFTABLE-RELATIONSHIP-MIB::cviRoutedVlanIfIndex[34][1] 7
	# 
	# key = [ifIndex]
	# IF-MIB::ifAlias[7] S078 (MINI-C L1)
	def __process_results(self, device, data, results):
		ifindex = None
		vlans = snmp.get_values2(results, "cviRoutedVlanIfIndex")
		for (key, candidate) in vlans.items():
			(vlan, vindex) = key
			if vlan == '34':
				ifindex = candidate
				break
				
		target = 'entities:%s' % device.admin_ip
		if ifindex:
			aliases = snmp.get_values1(results, "ifAlias")
			for (candidate, alias) in aliases.items():
				if candidate == ifindex:
					if 'S023' in alias:
						close_alert(data, target, key = 'inverted')
					elif 'S078' in alias:
						open_alert(data, target, key = 'inverted', mesg = 'VLANs need to be inverted.', resolution = 'Run `./DCI_ARL.sh 12 invert`', kind = 'error')
					else:
						open_alert(data, target, key = 'inverted', mesg = 'Expected interface S023 or S078 on VLAN 4 but found %s.' % alias, resolution = '', kind = 'error')
					return
		open_alert(data, target, key = 'inverted', mesg = 'Failed to find VLAN 4.', resolution = '', kind = 'error')
		
# Parse command line.
parser = argparse.ArgumentParser(description = "Uses snmp to verify that CAP2-RTR VLAN is inverted.")
parser.add_argument("--dont-put", dest = 'put', action='store_false', default=True, help = 'log results instead of PUTing them')
parser.add_argument("--duration", action='store', default=float('inf'), type=float, metavar='SECS', help = 'amount of time to poll (for testing)')
parser.add_argument("--stdout", action='store_true', default=False, help = 'log to stdout instead of snmp-modeler.log')
parser.add_argument("--verbose", "-v", action='count', help = 'print extra information')
parser.add_argument("--version", "-V", action='version', version='%(prog)s 0.1')	# TODO: keep this version synced up with the gnos version
parser.add_argument("config", metavar = "CONFIG-FILE", help = "path to json formatted configuration file")
env.options = parser.parse_args()

# Configure logging.
configure_logging(env.options.stdout, 'invert-modeler.log')

try:
	# Read config info.
	with open(env.options.config, 'r') as f:
		env.config = json.load(f)
		
	poller = Poll()
	poller.run()
except:
	env.logger.error("invert-modeler failed", exc_info = True)
