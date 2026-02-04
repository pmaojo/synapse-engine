import json
import os
import glob
from typing import List, Dict
from pathlib import Path
from dotenv import load_dotenv
import litellm
from sentence_transformers import SentenceTransformer
from qdrant_client import QdrantClient
from qdrant_client.http import models
import uuid

# Cargar variables de entorno
load_dotenv()

class RAGTeacher:
    def __init__(self, ontology_text: str, model: str = "gemini/gemini-2.5-flash"):
        self.ontology_text = ontology_text
        self.model = model or os.getenv("TEACHER_MODEL", "gemini/gemini-2.5-flash")
        
        # RAG Components
        self.encoder = SentenceTransformer('all-MiniLM-L6-v2')

        # Configure Qdrant Client
        qdrant_url = os.getenv("QDRANT_URL")
        if qdrant_url:
            self.qdrant = QdrantClient(url=qdrant_url)
        else:
            # Fallback to local storage (distinct from core to avoid conflicts if running locally)
            self.qdrant = QdrantClient(path="./qdrant_teacher_storage")

        self.collection_name = "teacher_docs"
        
        self._setup_vector_db()
        
    def _setup_vector_db(self):
        # Solo crear si no existe
        collections = self.qdrant.get_collections().collections
        exists = any(c.name == self.collection_name for c in collections)
        
        if not exists:
            self.qdrant.create_collection(
                collection_name=self.collection_name,
                vectors_config=models.VectorParams(size=384, distance=models.Distance.COSINE)
            )

    def index_documents(self, doc_dir: str = "documents"):
        """Index text files from directory (incremental)"""
        files = glob.glob(f"{doc_dir}/*.txt") + glob.glob(f"{doc_dir}/*.md")
        print(f"Escaneando {len(files)} documentos en {doc_dir}...")
        
        # Verificar qué ya está indexado
        # (Simplificación: consultamos por nombre de archivo si es posible, 
        # o simplemente confiamos en el scroll. Para hacerlo robusto, usamos scroll)
        
        # Recuperar todos los sources existentes
        existing_sources = set()
        try:
            # Scroll para obtener metadatos (limitado a 10000 chunks para este script simple)
            res = self.qdrant.scroll(
                collection_name=self.collection_name,
                scroll_filter=None,
                limit=10000,
                with_payload=True,
                with_vectors=False
            )[0]
            for point in res:
                if "source" in point.payload:
                    existing_sources.add(point.payload["source"])
        except Exception:
            pass # Colección vacía o error
            
        print(f"Documentos ya indexados: {len(existing_sources)} chunks/archivos detectados.")
        
        new_files = 0
        for file_path in files:
            filename = os.path.basename(file_path)
            
            # Si ya procesamos este archivo (basado en nombre), saltar
            # Nota: Esto es una heurística simple. Si el archivo cambia, no se actualiza.
            # Para producción real, usaríamos hashes del contenido.
            if filename in existing_sources:
                continue
                
            print(f"Indexando nuevo archivo: {filename}...")
            with open(file_path, 'r', encoding='utf-8') as f:
                text = f.read()
                
            # Split into chunks (simple paragraph split)
            chunks = [c.strip() for c in text.split('\n\n') if len(c.strip()) > 50]
            
            if not chunks:
                continue
                
            # Embed and store
            vectors = self.encoder.encode(chunks)
            points = [
                models.PointStruct(
                    id=str(uuid.uuid4()),
                    vector=v.tolist(),
                    payload={"text": chunk, "source": filename}
                )
                for chunk, v in zip(chunks, vectors)
            ]
            
            self.qdrant.upsert(collection_name=self.collection_name, points=points)
            new_files += 1
            
        if new_files > 0:
            print(f"✅ Indexación completada. {new_files} archivos nuevos añadidos.")
        else:
            print("✅ Todo actualizado. No hay archivos nuevos.")

    def retrieve_context(self, query: str, top_k: int = 3) -> str:
        """Retrieve relevant context for a query"""
        query_vector = self.encoder.encode(query).tolist()
        hits = self.qdrant.search(
            collection_name=self.collection_name,
            query_vector=query_vector,
            limit=top_k
        )
        return "\n\n".join([hit.payload['text'] for hit in hits])

    def generate_triples(self, text_chunk: str) -> Dict:
        """
        Generate triples using RAG + LLM
        """
        # Reuse the robust process_document logic
        return self.process_document(text_chunk)
        
    def generate_prompt(self, text_chunk: str) -> str:
        """
        Generates the prompt for the LLM to extract triples.
        """
        return f"""
        Actúa como un experto en Ontologías OWL y Agricultura.
        
        TU TAREA:
        Extraer triples semánticos (Sujeto, Predicado, Objeto) del texto proporcionado.
        
        REGLAS:
        1. Usa SOLO conceptos y propiedades válidos en el contexto de agricultura.
        2. El formato de salida debe ser JSON puro: [["Sujeto", "Predicado", "Objeto"], ...]
        3. Si no hay información relevante, devuelve [].
        4. Intenta alinear los términos con clases estándar.
        5. NO incluyas markdown.
        
        ONTOLOGÍA (Resumen):
        {self.ontology_text}
        
        TEXTO A PROCESAR:
        "{text_chunk}"
        
        SALIDA JSON:
        """

    def process_document(self, text: str) -> Dict:
        """
        Llama al LLM Maestro usando LiteLLM con reintentos de modelos.
        """
        prompt = self.generate_prompt(text)
        
        # Lista de modelos candidatos a probar (Actualizada con LiteLLM Docs)
        candidate_models = [
            self.model, # El configurado por defecto
            "gemini/gemini-2.5-pro-latest",
            "gemini/gemini-2.5-flash",
            "gemini/gemini-2.5-pro",
            "gemini/gemini-2.0-flash-exp",
            "gemini/gemini-2.5-flash-preview-09-2025",
            "gemini/gemini-1.0-pro",
            "gemini/gemini-2.5-flash"
        ]
        
        # Eliminar duplicados manteniendo orden
        candidate_models = list(dict.fromkeys(candidate_models))
        
        last_error = None
        
        for model in candidate_models:
            try:
                print(f"Consultando al Maestro ({model})...")
                response = litellm.completion(
                    model=model,
                    messages=[{"role": "user", "content": prompt}],
                    temperature=0.1
                )
                
                content = response.choices[0].message.content.strip()
                if content.startswith("```json"):
                    content = content.replace("```json", "").replace("```", "")
                
                triples = json.loads(content)
                return {"text": text, "triples": triples, "source": f"llm_teacher_{model}"}
                
            except Exception as e:
                print(f"⚠️ Falló {model}: {str(e)[:100]}...")
                last_error = e
                continue # Probar siguiente modelo
        
        # Si todos fallan, usar fallback
        print(f"❌ Todos los modelos fallaron. Último error: {last_error}")
        print("⚠️ Usando Fallback Heurístico...")
            
        # Fallback: Extracción basada en reglas simples para demostración
        fallback_triples = []
        lower_text = text.lower()
        
        # Reglas simples basadas en la ontología agriculture.owl
        if "bosque de alimentos" in lower_text or "food forest" in lower_text:
            fallback_triples.append(["FoodForest", "isA", "PermacultureSystem"])
            fallback_triples.append(["FoodForest", "mimics", "NaturalForest"])
        
        if "swales" in lower_text or "zanjas" in lower_text:
            fallback_triples.append(["Swales", "capture", "Water"])
            fallback_triples.append(["Swales", "prevent", "Erosion"])
            
        if "compost" in lower_text:
            fallback_triples.append(["Compost", "improves", "SoilStructure"])
            fallback_triples.append(["Compost", "provides", "Nutrients"])
            
        if "cobertura" in lower_text or "cover crop" in lower_text:
            fallback_triples.append(["CoverCrop", "protects", "Soil"])
        
        if "permacultura" in lower_text:
            fallback_triples.append(["Permaculture", "hasEthic", "EarthCare"])
            fallback_triples.append(["Permaculture", "hasEthic", "PeopleCare"])
        
        return {
            "text": text, 
            "triples": fallback_triples, 
            "source": "fallback_rules",
            "note": "Generated via fallback rules due to API error"
        }

def main():
    # 1. Setup
    ontology_context = """
    Classes: FoodForest, Swales, CoverCrop, Compost, PermacultureSystem
    Properties: hasComponent, improves, produces, isA, protects
    """
    teacher = RAGTeacher(ontology_context)
    
    # 2. Indexar documentos (si existen)
    teacher.index_documents()
    
    # 3. Generar datos iterando sobre los documentos indexados
    # Recuperamos todos los chunks indexados para procesarlos
    # (En un caso real, iteraríamos sobre los documentos fuente)
    
    # MOCK: Si no hay documentos, usamos textos de ejemplo
    raw_texts = [
        "Un bosque de alimentos es una técnica de permacultura.",
        "El compost mejora el suelo."
    ]
    
    distilled_data = []
    for text in raw_texts:
        res = teacher.generate_triples(text)
        if "error" not in res:
            distilled_data.append(res)
            print(f"✅ Generado: {res['triples']}")
            
    # 4. Guardar
    if distilled_data:
        with open("data/train_distilled.jsonl", "w", encoding="utf-8") as f:
            for ex in distilled_data:
                f.write(json.dumps(ex, ensure_ascii=False) + "\n")
        print(f"\n✅ Guardado en data/train_distilled.jsonl")

if __name__ == "__main__":
    main()
