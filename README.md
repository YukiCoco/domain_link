# domain_link
一个小工具，可将一个域名绑定到另一个域名的 A 记录。  
有这样一个需求：一个动态域名被污染，但是不得不使用它，所以只能将自己的域名与之绑定。  
只是方便自己用的，所以没有做过多的异常处理。

## 使用


### 配置
#### config.yml

```yaml
# 域名的 Zone ID，可在仪表盘看到
zone_identifier: 
# Global API KEY https://dash.cloudflare.com/profile/api-tokens
api_key: 
# CloudFlare 的登录邮箱
account_email: 
# 查询目标域名 DNS 的更新时间
sleep_duration: 30
domains:
  # name 为 CloudFlare 要更新的域名
- name: sh-cu.foo.com
  # origin_name 为要与目标绑定的域名
  origin_name: sh-cu.bar.com
  # ip 为目标域名当前 IP
  ip: 114.114.114.114
- name: sh-cn2.foo.com
  origin_name: sh-cn2.bar.com
  ip: 8.8.8.8
```
