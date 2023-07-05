
user=xxx
passwd=xxx

generator=scripts/toml_generator.py
runner=scripts/toml_runner.py

# arg1: template filepath
# arg2: output directory
# arg3: toml arguments dictory
# arg4: evaluation logs directory

# Run one command in one second in .toml 
define sep_bench
	mkdir -p $(2) $(4)
	python3 $(generator) -f $(1) -o $(2) -d $(3)
	python3 $(runner) -i $(2) -l $(4) -u $(user) -p $(passwd) -s True
endef

# Run all commands in .toml simutaneously
define bench
	mkdir -p $(2) $(4)
	python3 $(generator) -f $(1) -o $(2) -d $(3)
	python3 $(runner) -i $(2) -l $(4) -u $(user) -p $(passwd) -s False
endef


# clients=['val00', 'val01', 'val02', 'val03']
clients=['val00', 'val01']
bonus_clients=['val02', 'val03']
lat_client=['val00']
rdma_server=['pro2']
rdma_srv_ip='192.168.12.10'
snic_server=['pro1']
snic_srv_ip='192.168.12.138'
soc=['snic-pro1']
soc_ip='192.168.12.114'
rdma_server_nic=1
snic_server_nic=0
client_nic=0
parent='~/rocc'

x86_root='smartnic-bench'
soc_root='smartnic-bench/soc'

rdma_machines={'server': ${rdma_server}, 'client': ${clients}}
rdma_lat_machines={'server': ${rdma_server}, 'client': ${lat_client}}
snic_machines={'server': ${snic_server}, 'client': ${clients}}
snic_lat_machines={'server': ${snic_server}, 'client': ${lat_client}}
soc_machines={'server': ${soc}, 'client': ${clients}}
soc_lat_machines={'server': ${soc}, 'client': ${lat_client}}
soc_intranode_machines={'server': ${soc}, 'client': ${snic_server}}
snic_1_2_machines={'server': ${snic_server}, 'soc': ${soc}, \
'host_client': ${clients}, 'soc_client': ${bonus_clients}}
snic_1_3_machines={'server': ${snic_server}, 'soc': ${soc}, \
'client': ${clients}}

thpt_lat_results=throughput_latency/scripts/results
payloads=[0,16,32,64,128,256,512,1024]

tem_read_thpt=throughput_latency/scripts/templates/thpt/read.toml
tem_read_lat=throughput_latency/scripts/templates/lat/read.toml
tem_1_2_read_thpt=throughput_latency/scripts/templates/thpt/1_2_read.toml
tem_1_3_read_thpt=throughput_latency/scripts/templates/thpt/1_3_read.toml
tem_write_thpt=throughput_latency/scripts/templates/thpt/write.toml
tem_write_lat=throughput_latency/scripts/templates/lat/write.toml
tem_1_2_write_thpt=throughput_latency/scripts/templates/thpt/1_2_write.toml
tem_1_3_write_thpt=throughput_latency/scripts/templates/thpt/1_3_write.toml
tem_rpc_thpt=throughput_latency/scripts/templates/thpt/rpc.toml
tem_rpc_lat=throughput_latency/scripts/templates/lat/rpc.toml
tem_1_2_rpc_thpt=throughput_latency/scripts/templates/thpt/1_2_rpc.toml
tem_1_3_rpc_thpt=throughput_latency/scripts/templates/thpt/1_3_rpc.toml

rdma_args="{'hosts': ${rdma_machines}, \
'placeholder': {'payload': ${payloads}, \
'server_ip': ${rdma_srv_ip}, \
'server_root': ${x86_root}, \
'server_nic': ${rdma_server_nic}, \
'client_root': ${x86_root}, \
'client_nic': ${client_nic}}, \
'path': ${parent}}"
rdma_lat_args="{'hosts': ${rdma_lat_machines}, \
'placeholder': {'payload': ${payloads}, \
'server_ip': ${rdma_srv_ip}, \
'server_root': ${x86_root}, \
'server_nic': ${rdma_server_nic}, \
'client_root': ${x86_root}, \
'client_nic': ${client_nic}}, \
'path': ${parent}}"

snic_1_args="{'hosts': ${snic_machines}, \
'placeholder': {'payload': ${payloads}, \
'server_ip': ${snic_srv_ip}, \
'server_root': ${x86_root}, \
'server_nic': ${snic_server_nic}, \
'client_root': ${x86_root}, \
'client_nic': ${client_nic}}, \
'path': ${parent}}"
snic_1_lat_args="{'hosts': ${snic_lat_machines}, \
'placeholder': {'payload': ${payloads}, \
'server_ip': ${snic_srv_ip}, \
'server_root': ${x86_root}, \
'server_nic': ${snic_server_nic}, \
'client_root': ${x86_root}, \
'client_nic': ${client_nic}}, \
'path': ${parent}}"

snic_2_args="{'hosts': ${soc_machines}, \
'placeholder': {'payload': ${payloads}, \
'server_ip': ${soc_ip}, \
'server_root': ${soc_root}, \
'server_nic': ${snic_server_nic}, \
'client_root': ${x86_root}, \
'client_nic': ${client_nic}}, \
'path': ${parent}}"
snic_2_lat_args="{'hosts': ${soc_lat_machines}, \
'placeholder': {'payload': ${payloads}, \
'server_ip': ${soc_ip}, \
'server_root': ${soc_root}, \
'server_nic': ${snic_server_nic}, \
'client_root': ${x86_root}, \
'client_nic': ${client_nic}}, \
'path': ${parent}}"

snic_3_args="{'hosts': ${soc_intranode_machines}, \
'placeholder': {'payload': ${payloads}, \
'server_ip': ${soc_ip}, \
'server_root': ${soc_root}, \
'server_nic': ${snic_server_nic}, \
'client_root': ${x86_root}, \
'client_nic': ${client_nic}}, \
'path': ${parent}}"

snic_1_2_args="{'hosts': ${snic_1_2_machines}, \
'placeholder': {'payload': ${payloads}, \
'server_ip': ${snic_srv_ip}, \
'server_root': ${x86_root}, \
'server_nic': ${snic_server_nic}, \
'soc_ip': ${soc_ip}, \
'soc_root': ${soc_root}, \
'soc_nic': 0, \
'host_client_root': ${x86_root}, \
'soc_client_root': ${x86_root}', \
'client_nic': ${client_nic}}, \
'path': ${parent}}"

snic_1_3_args="{'hosts': ${snic_1_3_machines}, \
'placeholder': {'payload': ${payloads}, \
'server_ip': ${snic_srv_ip}, \
'server_root': ${x86_root}, \
'server_nic': ${snic_server_nic}, \
'soc_ip': ${soc_ip}, \
'soc_root': ${soc_root}, \
'soc_nic': 0, \
'client_root': ${x86_root}, \
'client_nic': ${client_nic}}, \
'path': ${parent}}"

rdma_read_thpt:
	$(call bench, \
	${tem_read_thpt}, \
	${thpt_lat_results}/rdma_read_thpt, \
	${rdma_args}, \
	${thpt_lat_results}/rdma_read_thpt/logs)
rdma_read_lat:
	$(call bench, \
	${tem_read_lat}, \
	${thpt_lat_results}/rdma_read_lat, \
	${rdma_lat_args}, \
	${thpt_lat_results}/rdma_read_lat/logs)

snic_1_read_thpt:
	$(call bench, \
	${tem_read_thpt}, \
	${thpt_lat_results}/snic_1_read_thpt, \
	${snic_1_args}, \
	${thpt_lat_results}/snic_1_read_thpt/logs)
snic_1_read_lat:
	$(call bench, \
	${tem_read_lat}, \
	${thpt_lat_results}/snic_1_read_lat, \
	${snic_1_lat_args}, \
	${thpt_lat_results}/snic_1_read_lat/logs)

snic_2_read_thpt:
	$(call bench, \
	${tem_read_thpt}, \
	${thpt_lat_results}/snic_2_read_thpt, \
	${snic_2_args}, \
	${thpt_lat_results}/snic_2_read_thpt/logs)
snic_2_read_lat:
	$(call bench, \
	${tem_read_lat}, \
	${thpt_lat_results}/snic_2_read_lat, \
	${snic_2_lat_args}, \
	${thpt_lat_results}/snic_2_read_lat/logs)

snic_3_read_thpt:
	$(call bench, \
	${tem_read_thpt}, \
	${thpt_lat_results}/snic_3_read_thpt, \
	${snic_3_args}, \
	${thpt_lat_results}/snic_3_read_thpt/logs)
snic_3_read_lat:
	$(call bench, \
	${tem_read_lat}, \
	${thpt_lat_results}/snic_3_read_lat, \
	${snic_3_args}, \
	${thpt_lat_results}/snic_3_read_lat/logs)

snic_1_2_read_thpt:
	$(call bench, \
	${tem_1_2_read_thpt}, \
	${thpt_lat_results}/snic_1_2_read_thpt, \
	${snic_1_2_args}, \
	${thpt_lat_results}/snic_1_2_read_thpt/logs)

snic_1_3_read_thpt:
	$(call bench, \
	${tem_1_3_read_thpt}, \
	${thpt_lat_results}/snic_1_3_read_thpt, \
	${snic_1_3_args}, \
	${thpt_lat_results}/snic_1_3_read_thpt/logs)

rdma_write_thpt:
	$(call bench, \
	${tem_write_thpt}, \
	${thpt_lat_results}/rdma_write_thpt, \
	${rdma_args}, \
	${thpt_lat_results}/rdma_write_thpt/logs)
rdma_write_lat:
	$(call bench, \
	${tem_write_lat}, \
	${thpt_lat_results}/rdma_write_lat, \
	${rdma_lat_args}, \
	${thpt_lat_results}/rdma_write_lat/logs)

snic_1_write_thpt:
	$(call bench, \
	${tem_write_thpt}, \
	${thpt_lat_results}/snic_1_write_thpt, \
	${snic_1_args}, \
	${thpt_lat_results}/snic_1_write_thpt/logs)
snic_1_write_lat:
	$(call bench, \
	${tem_write_lat}, \
	${thpt_lat_results}/snic_1_write_lat, \
	${snic_1_lat_args}, \
	${thpt_lat_results}/snic_1_write_lat/logs)

snic_2_write_thpt:
	$(call bench, \
	${tem_write_thpt}, \
	${thpt_lat_results}/snic_2_write_thpt, \
	${snic_2_args}, \
	${thpt_lat_results}/snic_2_write_thpt/logs)
snic_2_write_lat:
	$(call bench, \
	${tem_write_lat}, \
	${thpt_lat_results}/snic_2_write_lat, \
	${snic_2_lat_args}, \
	${thpt_lat_results}/snic_2_write_lat/logs)

snic_3_write_thpt:
	$(call bench, \
	${tem_write_thpt}, \
	${thpt_lat_results}/snic_3_write_thpt, \
	${snic_3_args}, \
	${thpt_lat_results}/snic_3_write_thpt/logs)
snic_3_write_lat:
	$(call bench, \
	${tem_write_lat}, \
	${thpt_lat_results}/snic_3_write_lat, \
	${snic_3_args}, \
	${thpt_lat_results}/snic_3_write_lat/logs)

snic_1_2_write_thpt:
	$(call bench, \
	${tem_1_2_write_thpt}, \
	${thpt_lat_results}/snic_1_2_write_thpt, \
	${snic_1_2_args}, \
	${thpt_lat_results}/snic_1_2_write_thpt/logs)

snic_1_3_write_thpt:
	$(call bench, \
	${tem_1_3_write_thpt}, \
	${thpt_lat_results}/snic_1_3_write_thpt, \
	${snic_1_3_args}, \
	${thpt_lat_results}/snic_1_3_write_thpt/logs)

rdma_rpc_thpt:
	$(call bench, \
	${tem_rpc_thpt}, \
	${thpt_lat_results}/rdma_rpc_thpt, \
	${rdma_args}, \
	${thpt_lat_results}/rdma_rpc_thpt/logs)
rdma_rpc_lat:
	$(call bench, \
	${tem_rpc_lat}, \
	${thpt_lat_results}/rdma_rpc_lat, \
	${rdma_lat_args}, \
	${thpt_lat_results}/rdma_rpc_lat/logs)

snic_1_rpc_thpt:
	$(call bench, \
	${tem_rpc_thpt}, \
	${thpt_lat_results}/snic_1_rpc_thpt, \
	${snic_1_args}, \
	${thpt_lat_results}/snic_1_rpc_thpt/logs)
snic_1_rpc_lat:
	$(call bench, \
	${tem_rpc_lat}, \
	${thpt_lat_results}/snic_1_rpc_lat, \
	${snic_1_lat_args}, \
	${thpt_lat_results}/snic_1_rpc_lat/logs)

snic_2_rpc_thpt:
	$(call bench, \
	${tem_rpc_thpt}, \
	${thpt_lat_results}/snic_2_rpc_thpt, \
	${snic_2_args}, \
	${thpt_lat_results}/snic_2_rpc_thpt/logs)
snic_2_rpc_lat:
	$(call bench, \
	${tem_rpc_lat}, \
	${thpt_lat_results}/snic_2_rpc_lat, \
	${snic_2_lat_args}, \
	${thpt_lat_results}/snic_2_rpc_lat/logs)

snic_3_rpc_thpt:
	$(call bench, \
	${tem_rpc_thpt}, \
	${thpt_lat_results}/snic_3_rpc_thpt, \
	${snic_3_args}, \
	${thpt_lat_results}/snic_3_rpc_thpt/logs)
snic_3_rpc_lat:
	$(call bench, \
	${tem_rpc_lat}, \
	${thpt_lat_results}/snic_3_rpc_lat, \
	${snic_3_args}, \
	${thpt_lat_results}/snic_3_rpc_lat/logs)

snic_1_2_rpc_thpt:
	$(call bench, \
	${tem_1_2_rpc_thpt}, \
	${thpt_lat_results}/snic_1_2_rpc_thpt, \
	${snic_1_2_args}, \
	${thpt_lat_results}/snic_1_2_rpc_thpt/logs)

snic_1_3_rpc_thpt:
	$(call bench, \
	${tem_1_3_rpc_thpt}, \
	${thpt_lat_results}/snic_1_3_rpc_thpt, \
	${snic_1_3_args}, \
	${thpt_lat_results}/snic_1_3_rpc_thpt/logs)
