    pub fn anonymize_entity(&mut self, detected: &DetectedEntity) -> Result<AnonymizedEntity> {
        let entity_type = self.extract_base_type(&detected.entity_type);
        
        let fake_value = match entity_type.as_str() {
            "email" => self.generate_fake_email(),
            "phone" => self.generate_fake_phone(),
            "ssn" => self.generate_fake_ssn(),
            "name" | "person_name" => self.generate_fake_name(),
            "ip_address" => self.generate_fake_ip(),
            "hostname" => self.generate_fake_hostname(),
            "node_name" => self.generate_fake_node_name(),
            "credit_card" => self.generate_fake_credit_card(),
            _ => {
                warn!("Unknown entity type '{}', using generic replacement", entity_type);
                format!("REDACTED_{}", entity_type.to_uppercase())
            }
        };