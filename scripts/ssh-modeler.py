#!/usr/bin/python
# Modeler designed for use on Linux devices that are not running SNMP. It's
# designed to be fairly minimal (if you want detailed information then get
# SNMP running).
import cgi, json, itertools, httplib, re, subprocess, sys, threading, time
from base_modeler import *

try:
	import argparse
except:
	sys.stderr.write("This script requires Python 2.7 or later\n")
	sys.exit(2)
	
logger = None
connection = None

class UName(object):
	def command(self):
		return 'uname -a'
	
	# Linux auto-fat 2.6.32-33-server #70-Ubuntu SMP Thu Jul 7 22:28:30 UTC 2011 x86_64 GNU/Linux
	def process(self, data, admin_ip, text, context):
		logger.debug("uname: '%s'" % text)
		target = 'entities:%s' % admin_ip
		add_details(data, target, 'OS', text, opened = 'always', sort_key = 'alpha', key = 'uname')
		
class Uptime(object):
	def command(self):
		return 'uptime'
		
	up_expr1 = re.compile(r'up \s+ (\d+) \s+ (min|sec|day)', re.MULTILINE | re.VERBOSE)
	up_expr2 = re.compile(r'up \s+ (\d+ : \d+) ,', re.MULTILINE | re.VERBOSE)
	load_expr = re.compile(r'load\ average: \s+ ([\d.]+)', re.MULTILINE | re.VERBOSE)
	
	# 04:43:21 up 0 min, load average: 1.02, 0.27, 0.09
	# 19:08:55 up  9:39, load average: 0.00, 0.01, 0.04
	# 07:13:42 up 52 days, 16:04,  3 users,  load average: 0.00, 0.00, 0.05
	def process(self, data, admin_ip, text, context):
		logger.debug("uptime: '%s'" % text)
		target = 'entities:%s' % admin_ip
		
		# Add an alert if the device has only been up a short time. There is potentially a lot
		# of variation here so, for now, we'll just match what we need. TODO: Better to try
		# and match everything so we have some hope of spotting problems. Even better
		# would be to use something more suitable for machine parsing (proc file?).
		match = re.search(Uptime.up_expr1, text)
		if match:
			if match.group(2) == 'sec' or int(match.group(1)) <= 1:
				# TODO: Can we add something helpful for resolution? Some log files to look at? A web site?
				open_alert(data, target, key = 'uptime', mesg = 'Device rebooted.', resolution = '', kind = 'error')
			else:
				close_alert(data, target, key = 'uptime')
			
		match = re.search(Uptime.up_expr2, text)
		if match:
			close_alert(data, target, key = 'uptime')
				
		# The load average is an average of the number of processes forced to wait
		# for CPU over the last 1, 5, and 15 minutes. We'll record the average for the 
		# last minute.
		match = re.search(Uptime.load_expr, text)
		if match:
			context['load averages'][admin_ip] = float(match.group(1))
		
class CpuInfo(object):
	# Note that this will count both CPUs and cores.
	def command(self):
		return 'cat /proc/cpuinfo | grep -E "[Pp]rocessor[^:alpha:]+:" | wc -l'
		
	# 1
	def process(self, data, admin_ip, text, context):
		logger.debug("cpuinfo: '%s'" % text)
		if text.isdigit():
			context['num_cores'][admin_ip] = int(text)
		
class DeviceRunner(object):
	def __init__(self, ip, ssh, handlers):
		self.ip = ip
		self.__ssh = ssh
		self.__handlers = handlers
		commands = map(lambda x: '%s' % x.command(), handlers)
		self.__command = '; echo \'^^^\'; '.join(commands)
		
	def run(self):
		self.results = None
		command = '%s%s "%s"' % (self.__ssh, self.ip, self.__command)
		logger.debug(command)
		try:
			result = run_process(command)
			parts = result.split('^^^')
			if len(parts) == len(self.__handlers):
				self.results = map(lambda x: x.strip(), parts)
			else:
				raise Exception("Expected %s results but found '%s'" % (len(self.__handlers), result))
		except:
			logger.error("Error executing `%s`" % command, exc_info = True)

class Poll(object):
	def __init__(self, args, config):
		self.__args = args
		self.__config = config
		self.__startTime = time.time()
		self.__last_time = None
		self.__handlers = [UName(), Uptime(), CpuInfo()]
		self.__context = {}
		self.__num_samples = 0
	
	def run(self):
		rate = self.__config['poll-rate']
		while True:
			self.__current_time = time.time()
			
			self.__context['load averages'] = {}	# admin ip => 1min load average
			self.__context['num_cores'] = {}		# admin_ip => number of cores
			
			threads = self.__spawn_threads()
			data = self.__process_threads(threads)
			self.__add_cpu_load_gauge(data)
			send_update(self.__config, connection, data)
			
			elapsed = time.time() - self.__current_time
			self.__last_time = self.__current_time
			self.__num_samples += 1
			logger.info('elapsed: %.1f seconds' % elapsed)
			if time.time() - self.__startTime < self.__args.duration:
				time.sleep(max(rate - elapsed, 5))
			else:
				break
				
	def __add_cpu_load_gauge(self, data):
		for (admin_ip, load) in self.__context['load averages'].items():
			if self.__context['num_cores'][admin_ip] != None:
				value = load/self.__context['num_cores'][admin_ip]
				target = 'entities:%s' % admin_ip
				level = None
				if value >= 0.90:
					level = 1
					style = 'gauge-bar-color:salmon'
				elif value >= 0.75:
					level = 2
					style = 'gauge-bar-color:darkorange'
				elif value >= 0.50:
					level = 3
					style = 'gauge-bar-color:skyblue'
				else:				# TODO: get rid of this case
					level = 4
					style = 'gauge-bar-color:green'
				if level:
					add_gauge(data, target, 'processor load', value, level, style, sort_key = 'y')
			
	def __process_threads(self, threads):
		data = {'modeler': 'snmp', 'entities': [], 'relations': [], 'labels': [], 'gauges': [], 'details': [], 'alerts': [], 'samples': [], 'charts': []}
		for thread in threads:
			target = 'entities:%s' % thread.ip
			close_alert(data, target, key = 'device down')
			if thread.results:
				assert len(thread.results) == len(self.__handlers)
				for i in xrange(0, len(thread.results)):
					self.__handlers[i].process(data, thread.ip, thread.results[i], self.__context)
			else:
				open_alert(data, target, key = 'device down', mesg = 'Device is down.', resolution = 'Check the power cable, power it on if it is off, check the IP address, verify routing.', kind = 'error')
		return data
	
	# It would be much better to actually do these within threads, but at least some Linux devices
	# seem to have very small limits on the number of outstanding ssh sessions. (When this happens
	# nothing is returned for random ssh sesssions).
	def __spawn_threads(self):
		threads = []
		for (name, device) in self.__config["devices"].items():
			if device['modeler'] == 'ssh-modeler.py':
				thread = DeviceRunner(device['ip'], device['ssh'], self.__handlers)
				thread.run()
				threads.append(thread)
		return threads

# Parse command line.
parser = argparse.ArgumentParser(description = "Uses ssh to model a network and sends the result to a gnos server.")
parser.add_argument("--dont-put", dest = 'put', action='store_false', default=True, help = 'log results instead of PUTing them')
parser.add_argument("--duration", action='store', default=float('inf'), type=float, metavar='SECS', help = 'amount of time to poll (for testing)')
parser.add_argument("--stdout", action='store_true', default=False, help = 'log to stdout instead of ssh-modeler.log')
parser.add_argument("--verbose", "-v", action='count', help = 'print extra information')
parser.add_argument("--version", "-V", action='version', version='%(prog)s 0.1')	# TODO: keep this version synced up with the gnos version
parser.add_argument("config", metavar = "CONFIG-FILE", help = "path to json formatted configuration file")
args = parser.parse_args()

# Configure logging.
logger = configure_logging(args, 'ssh-modeler.log')

# Read config info.
config = None
with open(args.config, 'r') as f:
	config = json.load(f)
	
if args.put:
	address = "%s:%s" % (config['server'], config['port'])
	connection = httplib.HTTPConnection(address, strict = True, timeout = 10)

try:
	poller = Poll(args, config)
	poller.run()
finally:
	if connection:
		connection.close()
