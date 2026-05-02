You are a PII detector. Extract all personally identifiable information from the text below.

TEXT: "{text}"

Return ONLY valid JSON in this exact format:
{"entities": [{"type": "TYPE", "value": "EXACT_TEXT", "start": 0, "end": 0, "confidence": 0.9}]}

Entity types: person_name, email, phone, ssn, ip_address, credit_card

If no PII found, return: {"entities": []}

JSON only, no explanation:
