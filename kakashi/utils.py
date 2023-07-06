import os
import sys

import jsonlines
import yaml
from langchain.schema import Document


class DocsJSONLLoader:
    """
    Loader for documents in JSONL format.

    Args:
        file_path (str): Path to the JSONL file to load.
    """

    def __init__(self, file_path: str):
        self.file_path = file_path

    def load(self):
        """
        Loads the documents from the file path specified during initialization.

        Returns:
            A list of Document objects.
        """
        with jsonlines.open(self.file_path) as reader:
            documents = []
            for obj in reader:
                page_content = obj.get("text", "")
                metadata = {
                    "title": obj.get("title", ""),
                    "repo_owner": obj.get("repo_owner", ""),
                    "repo_name": obj.get("repo_name", ""),
                }
                documents.append(Document(page_content=page_content, metadata=metadata))
        return documents


def load_config():
    """
    Loads the application configuration from the 'config.yaml' file.

    Returns:
        A dictionary with the application configuration.
    """
    root_dir = os.path.dirname(os.path.abspath(__file__))
    with open(os.path.join(root_dir, "config.yaml")) as stream:
        try:
            return yaml.safe_load(stream)
        except yaml.YAMLError as exc:
            print(exc)


def get_openai_api_key():
    """
    Gets the OpenAI API key from the environment. If it's not available, it stops the program execution.

    Returns:
        The OpenAI API key.
    """
    openai_api_key = os.getenv("OPENAI_API_KEY")
    if not openai_api_key:
        print("Please set OPENAI_API_KEY as an environment variable.")
        sys.exit()
    return openai_api_key


def get_cohere_api_key():
    """
    Gets the Cohere API key from the environment. If it's not available, it asks the user to enter it.

    Returns:
        The Cohere API key.
    """
    cohere_api_key = os.getenv("COHERE_API_KEY")
    if not cohere_api_key:
        cohere_api_key = input("Please enter your COHERE_API_KEY: ")
    return cohere_api_key


def get_file_path():
    """
    Gets the path to the JSONL database file specified in the application configuration.

    Returns:
        The path to the JSONL database file.
    """
    config = load_config()

    root_dir = os.path.dirname(os.path.abspath(__file__))
    parent_dir = os.path.join(root_dir, "..")

    return os.path.join(parent_dir, config["jsonl_database_path"])


def get_query_from_user() -> str:
    """
    Asks the user for a query.

    Returns:
        The query entered by the user.
    """
    try:
        query = input()
        return query
    except EOFError:
        print("Error: Unexpected input. Please try again.")
        return get_query_from_user()


def create_dir(path: str) -> None:
    """
    Creates a directory if it doesn't exist.

    Args:
        path (str): Path of the directory to create.
    """
    if not os.path.exists(path):
        os.makedirs(path)


def remove_existing_file(file_path: str) -> None:
    """
    Deletes a file if it exists.

    Args:
        file_path (str): Path of the file to delete.
    """
    if os.path.exists(file_path):
        os.remove(file_path)
