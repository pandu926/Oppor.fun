import os,json,secrets,subprocess,pathlib
root=pathlib.Path('/root/.config/oppor/production');tls=root/'tls';tls.mkdir(mode=0o755,parents=True,exist_ok=True);os.chmod(root,0o700)
def openssl(*args):subprocess.run(['openssl',*args],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
if not (tls/'ca.crt').exists():
 openssl('req','-x509','-newkey','rsa:3072','-nodes','-keyout',str(tls/'ca.key'),'-out',str(tls/'ca.crt'),'-days','3650','-subj','/CN=Oppor private service CA')
 os.chmod(tls/'ca.key',0o600)
for name,uid in [('postgres',999),('redis',999)]:
 folder=tls/name;folder.mkdir(mode=0o755,exist_ok=True)
 key=folder/'server.key';crt=folder/'server.crt';csr=folder/'server.csr';ext=folder/'server.ext'
 if not crt.exists():
  openssl('req','-new','-newkey','rsa:2048','-nodes','-keyout',str(key),'-out',str(csr),'-subj','/CN='+name)
  ext.write_text('subjectAltName=DNS:'+name+'\nextendedKeyUsage=serverAuth\n')
  openssl('x509','-req','-in',str(csr),'-CA',str(tls/'ca.crt'),'-CAkey',str(tls/'ca.key'),'-CAcreateserial','-out',str(crt),'-days','825','-extfile',str(ext))
  os.chown(key,uid,uid);os.chmod(key,0o600)
credentials=root/'service-credentials.json'
if not credentials.exists():
 d={k:secrets.token_hex(32) for k in ['postgres_admin_password','postgres_runtime_password','redis_password','seed_key','rate_key']}
 fd=os.open(credentials,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
 with os.fdopen(fd,'w') as f:json.dump(d,f)
d=json.loads(credentials.read_text())
(root/'database.env').write_text('POSTGRES_USER=oppor_owner\nPOSTGRES_DB=oppor\nPOSTGRES_PASSWORD='+d['postgres_admin_password']+'\n');os.chmod(root/'database.env',0o600)
(tls/'redis'/'redis.conf').write_text('port 0\ntls-port 6379\ntls-cert-file /tls/server.crt\ntls-key-file /tls/server.key\ntls-ca-cert-file /tls/ca.crt\ntls-auth-clients no\nprotected-mode yes\nbind 0.0.0.0\nrequirepass '+d['redis_password']+'\nappendonly yes\nappendfsync everysec\nmaxmemory 256mb\nmaxmemory-policy noeviction\n');os.chown(tls/'redis'/'redis.conf',999,999);os.chmod(tls/'redis'/'redis.conf',0o600)
for name in ['postgres','redis']:(tls/name/'ca.crt').write_bytes((tls/'ca.crt').read_bytes())
(tls/'postgres'/'pg_hba.conf').write_text('local all all trust\nhostssl all all all scram-sha-256\nhostnossl all all all reject\n')
print('Production service secrets and private TLS certificates provisioned.')

# Compose validates referenced env-file paths even before application activation.
for name in ['application.env', 'migration.env']:
 path = root/name
 if not path.exists():
  fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
  os.close(fd)
backup_dir = pathlib.Path('/var/lib/oppor/backups')
backup_dir.mkdir(mode=0o700, parents=True, exist_ok=True)
key_path = root/'backup-age.key'
if not key_path.exists():
 key = subprocess.run(['age-keygen'], check=True, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL).stdout
 fd = os.open(key_path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
 with os.fdopen(fd, 'wb') as output: output.write(key)
