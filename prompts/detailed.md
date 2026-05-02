# Detailed PII Detection Prompt (requires 3B+ model)

JSON_MODE_ONLY

TEXT: "{text}"

OUTPUT_REQUIREMENT: Return ONLY valid JSON. NO explanations. NO text. NO markdown. ONLY JSON.

FORMAT: {"entities": [{"type": "person_name", "value": "found_name", "start": 0, "end": 0, "confidence": 0.9}]}

EMPTY_RESULT: {"entities": []}

CRITICAL: Find ALL entities in the text. Scan for multiple entity types.

## Entity Types

### 1. person_name
Full names of people, first and last names together.
**EXAMPLES:** Sarah Johnson, John Smith, Maria Garcia, David Chen

### 2. email
Email addresses in any format.
**EXAMPLES:** sarah@company.com, john.smith@example.org

### 3. phone
Phone numbers in various formats.
**EXAMPLES:** 555-123-4567, (555) 123-4567, +1-555-123-4567

### 4. ssn
Social Security Numbers.
**EXAMPLES:** 123-45-6789

### 5. ip_address
IP addresses (IPv4 and IPv6).
**EXAMPLES:** 192.168.1.1, 10.0.0.1

### 6. credit_card
Credit card numbers.
**EXAMPLES:** 4111-1111-1111-1111

### 7. hostname
Computer hostnames, server names, system identifiers.
**EXAMPLES:** ubuntu-linux-2404, web-server-prod-01, wazuh-manager-01

### 8. node_name
Cluster nodes, compute nodes in distributed systems.
**EXAMPLES:** node01, worker-1, master-03, compute-node-5

## Detection Rules

- Look for "Node:" followed by any identifier → node_name
- Any word containing "node" + digits → node_name
- OS names with dashes/numbers (ubuntu, centos, debian) → hostname
- "Name:" followed by system identifier → hostname

Return valid JSON only:
