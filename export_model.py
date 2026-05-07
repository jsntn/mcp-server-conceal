#!/usr/bin/env python3
"""Export a HuggingFace NER model to ONNX format for mcp-server-conceal.

Usage:
    python3 export_model.py [MODEL_NAME] [OUTPUT_DIR]

Defaults:
    MODEL_NAME: dslim/bert-base-NER
    OUTPUT_DIR: ~/.local/share/mcp-server-conceal/
"""

import sys
from pathlib import Path

def main():
    model_name = sys.argv[1] if len(sys.argv) > 1 else "dslim/bert-base-NER"
    output_dir = Path(sys.argv[2] if len(sys.argv) > 2 else Path.home() / ".local/share/mcp-server-conceal")
    output_dir.mkdir(parents=True, exist_ok=True)

    print(f"Exporting {model_name} to ONNX...")

    from transformers import AutoTokenizer, AutoModelForTokenClassification
    import torch

    tokenizer = AutoTokenizer.from_pretrained(model_name)
    model = AutoModelForTokenClassification.from_pretrained(model_name)
    model.eval()

    # Save tokenizer in HuggingFace tokenizers format
    tokenizer.save_pretrained(str(output_dir))
    # Convert to fast tokenizer json if not already
    if hasattr(tokenizer, "backend_tokenizer"):
        tokenizer.backend_tokenizer.save(str(output_dir / "tokenizer.json"))

    # Export to ONNX
    dummy = tokenizer("Hello world", return_tensors="pt")
    onnx_path = output_dir / "model.onnx"

    torch.onnx.export(
        model,
        (dummy["input_ids"], dummy["attention_mask"]),
        str(onnx_path),
        input_names=["input_ids", "attention_mask"],
        output_names=["logits"],
        dynamic_axes={
            "input_ids": {0: "batch", 1: "seq"},
            "attention_mask": {0: "batch", 1: "seq"},
            "logits": {0: "batch", 1: "seq"},
        },
        opset_version=14,
    )

    # Save label list
    labels = [model.config.id2label[i] for i in range(len(model.config.id2label))]
    (output_dir / "labels.txt").write_text("\n".join(labels))

    print(f"\nExported to: {output_dir}")
    print(f"  Model:     {onnx_path} ({onnx_path.stat().st_size // 1024 // 1024}MB)")
    print(f"  Tokenizer: {output_dir / 'tokenizer.json'}")
    print(f"  Labels:    {labels}")
    print(f"\nAdd to mcp-server-conceal.toml:")
    print(f'  [ner]')
    print(f'  model_path = "{onnx_path}"')
    print(f'  tokenizer_path = "{output_dir / "tokenizer.json"}"')
    print(f'  labels = {labels}')


if __name__ == "__main__":
    main()
