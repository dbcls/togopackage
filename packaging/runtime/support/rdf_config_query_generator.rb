# frozen_string_literal: true

$LOAD_PATH.unshift(File.expand_path('../../../vendor/rdf-config/lib', __dir__))

require 'rdf-config'

module RDFConfigQueryGenerator
  module_function

  def generate_named_query(config_dir, query_name, parameter_names: [])
    config = RDFConfig::Config.new(File.expand_path(config_dir))
    opts = {
      sparql: query_name.to_s
    }
    unless parameter_names.empty?
      opts[:query] = parameter_names.map { |name| "#{name}={{{#{name}}}}" }
    end

    RDFConfig::SPARQL.new(config, opts).generate
  end

  def generate_model_query(config_dir, subject_name, attribute_name)
    config = RDFConfig::Config.new(File.expand_path(config_dir))
    RDFConfig::SPARQL.new(
      config,
      query: [subject_name.to_s, attribute_name.to_s]
    ).generate
  end
end
